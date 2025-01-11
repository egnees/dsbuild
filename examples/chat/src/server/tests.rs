use std::collections::BTreeSet;

use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};
use rand_pcg::Pcg64;
use rand_seeder::Seeder;

use dsbuild::{Address, Context, Message, Process, Sim};

use crate::{
    client::requests::{ClientRequest, ClientRequestKind},
    server::{
        event::{ChatEvent, ChatEventKind},
        messages::ServerMessage,
    },
    utils::sim::read_history,
};

use super::process::ServerProcess;

struct ClientStub {
    server: Address,
    req_id: u64,
    login: String,
    password: String,
}

impl ClientStub {
    pub fn new(server: Address) -> Self {
        Self {
            server,
            req_id: 0,
            login: "client".to_owned(),
            password: "password".to_owned(),
        }
    }

    pub fn new_with_auth_data(server: Address, login: String, password: String) -> Self {
        Self {
            server,
            req_id: 0,
            login,
            password,
        }
    }
}

impl Process for ClientStub {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        let kind = msg.get_data::<ClientRequestKind>().unwrap();
        let req = ClientRequest {
            id: self.req_id,
            client: self.login.clone(),
            password: self.password.clone(),
            time: None,
            kind,
            addr: None,
        };
        self.req_id += 1;
        let to = self.server.clone();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(req.into(), to, 5.0).await;
        });
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) {
        assert_eq!(from, self.server);
        ctx.send_local(msg);
    }
}

#[test]
fn state_works() {
    let mut sys = Sim::new(12345);
    let server_addr = Address {
        host: "server".to_owned(),
        port: 12345,
        process_name: "server".to_owned(),
    };
    sys.add_node_with_storage("server", &server_addr.host, server_addr.port, 1 << 20);
    sys.add_process("server", ServerProcess::default(), "server");

    sys.add_node("client", "client", 12345);
    sys.add_process("client", ClientStub::new(server_addr), "client");

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Create("chat".to_string()).into(),
    );

    sys.step_until_no_events();

    let messages = sys.read_local_messages("client", "client").unwrap();
    assert_eq!(messages.len(), 1);
    let response = messages[0].get_data::<ServerMessage>().unwrap();
    assert_eq!(response, ServerMessage::RequestResponse(0, Ok(())));

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Connect("chat".to_string()).into(),
    );

    sys.step_until_no_events();

    let messages = read_history(&mut sys, "client", "client");
    assert_eq!(messages.len(), 2);

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::SendMessage("msg".to_string()).into(),
    );

    sys.step_until_no_events();

    sys.send_local_message("client", "client", ClientRequestKind::Disconnect.into());

    sys.step_until_no_events();
    let _ = sys.read_local_messages("client", "client").unwrap();

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Connect("chat".to_string()).into(),
    );

    sys.step_until_no_events();

    let chat_history = read_history(&mut sys, "client", "client");

    assert_eq!(chat_history.len(), 5);

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::SendMessage("msg1".to_string()).into(),
    );
    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::SendMessage("msg2".to_string()).into(),
    );
    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::SendMessage("msg3".to_string()).into(),
    );
    sys.step_until_no_events();

    let new_events = read_history(&mut sys, "client", "client");

    assert_eq!(new_events.len(), 3);

    sys.shutdown_node("server");
    sys.step_until_no_events();

    sys.rerun_node("server");
    sys.add_process("server", ServerProcess::default(), "server");

    sys.send_local_message("client", "client", ClientRequestKind::Disconnect.into());
    sys.step_until_no_events();

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Connect("chat".to_string()).into(),
    );
    sys.step_until_no_events();

    let history = read_history(&mut sys, "client", "client");

    assert_eq!(history.len(), 10);
    for (i, event) in history.iter().enumerate() {
        assert_eq!(i as u64, event.seq);
    }
}

#[test]
fn state_multiple_users() {
    let mut chats = ["chat1", "chat2", "chat3", "chat4", "chat5"];
    let clients = ["client1", "client2", "client3", "client4", "client5"];

    let mut sys = Sim::new(12345);
    let server_addr = Address {
        host: "server".to_owned(),
        port: 12345,
        process_name: "server".to_owned(),
    };
    sys.add_node_with_storage("server", &server_addr.host, server_addr.port, 1 << 20);
    sys.add_process(
        &server_addr.process_name,
        ServerProcess::default(),
        "server",
    );

    for (i, client) in clients.iter().enumerate() {
        sys.add_node(client, client, 1111);
        let client_proc = ClientStub::new_with_auth_data(
            server_addr.clone(),
            client.to_string(),
            "pass123".to_string(),
        );
        sys.add_process(client, client_proc, client);

        sys.send_local_message(
            client,
            client,
            ClientRequestKind::Create(chats[i].to_string()).into(),
        );
    }

    sys.set_network_delays(0.5, 1.0);
    sys.set_network_drop_rate(0.05);

    sys.step_until_no_events();

    for iter in 0..10 {
        chats.shuffle(&mut Seeder::from(iter).make_rng::<Pcg64>());

        for (i, client) in clients.iter().enumerate() {
            sys.send_local_message(
                client,
                client,
                ClientRequestKind::Connect(chats[i].to_string()).into(),
            );
        }

        sys.step_until_no_events();

        for (i, client) in clients.iter().enumerate() {
            let msg: String = Seeder::from(iter ^ i)
                .make_rng::<Pcg64>()
                .sample_iter(&Alphanumeric)
                .take((iter + 1) * (i + 1))
                .map(char::from)
                .collect();
            sys.send_local_message(client, client, ClientRequestKind::SendMessage(msg).into());
        }

        sys.step_until_no_events();

        for client in clients.iter() {
            sys.send_local_message(client, client, ClientRequestKind::Disconnect.into());
        }

        sys.step_until_no_events();
    }

    let client = clients[0];
    sys.read_local_messages(client, client).unwrap();

    for chat in chats.iter() {
        sys.send_local_message(
            client,
            client,
            ClientRequestKind::Connect(chat.to_string()).into(),
        );

        sys.step_until_no_events();

        let history = sys
            .read_local_messages(client, client)
            .unwrap()
            .into_iter()
            .map(|msg| msg.get_data::<ServerMessage>().unwrap())
            .filter(|msg| match msg {
                ServerMessage::RequestResponse(_, res) => {
                    assert_eq!(*res, Ok(()));
                    false
                }
                ServerMessage::ChatEvent(_, _) => true,
            })
            .map(|msg| match msg {
                ServerMessage::RequestResponse(_, _) => panic!("impossible"),
                ServerMessage::ChatEvent(_, event) => event,
            })
            .collect::<BTreeSet<ChatEvent>>();
        assert_eq!(history.len(), 1 + 10 + 10 + 10 + 1);

        sys.send_local_message(client, client, ClientRequestKind::Disconnect.into());

        sys.step_until_no_events();
    }
}

struct ReplicaNotifiedClientStub {
    name: String,
    password: String,
    server1: Address,
    server2: Address,
    req_id: u64,
}

impl ReplicaNotifiedClientStub {
    pub fn new(name: String, password: String, server1: Address, server2: Address) -> Self {
        Self {
            name,
            password,
            server1,
            server2,
            req_id: 0,
        }
    }
}

impl Process for ReplicaNotifiedClientStub {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        let kind = msg.get_data::<ClientRequestKind>().unwrap();
        let req = ClientRequest {
            id: self.req_id,
            client: self.name.clone(),
            password: self.password.clone(),
            time: None,
            kind,
            addr: None,
        };
        self.req_id += 1;
        let to1 = self.server1.clone();
        let to2 = self.server2.clone();
        ctx.clone().spawn(async move {
            let msg: Message = req.into();
            let send_result = ctx.send_with_ack(msg.clone(), to1, 5.0).await;
            if send_result.is_err() {
                ctx.send_with_ack(msg, to2, 5.0).await.unwrap();
            }
        });
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) {
        assert!(from == self.server1 || from == self.server2);
        ctx.send_local(msg);
    }
}

#[test]
fn replication_works() {
    let mut sys = Sim::new(543210);

    sys.add_node("client", "client", 0);
    sys.add_node_with_storage("server1", "server1", 0, 4096);
    sys.add_node_with_storage("server2", "server2", 0, 4096);

    sys.connect_node_to_network("client1");
    sys.connect_node_to_network("server1");
    sys.connect_node_to_network("server2");
    sys.set_network_delays(0.5, 1.0);

    sys.add_process(
        "server1",
        ServerProcess::new_with_replica(Address::new_ref("server2", 0, "server2")),
        "server1",
    );

    sys.add_process(
        "server2",
        ServerProcess::new_with_replica(Address::new_ref("server1", 0, "server1")),
        "server2",
    );

    sys.add_process(
        "client",
        ReplicaNotifiedClientStub::new(
            "client".to_owned(),
            "pass123".to_owned(),
            Address::new_ref("server1", 0, "server1"),
            Address::new_ref("server2", 0, "server2"),
        ),
        "client",
    );

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Create("chat1".to_owned()).into(),
    );

    sys.step_until_no_events();

    let msgs = sys.read_local_messages("client", "client").unwrap();
    assert_eq!(msgs.len(), 1);

    sys.shutdown_node("server1");

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );

    sys.step_until_no_events();

    let history = read_history(&mut sys, "client", "client");
    assert_eq!(history[0].kind, ChatEventKind::Created());
    assert_eq!(history[1].kind, ChatEventKind::Connected());

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::SendMessage("hello".to_owned()).into(),
    );

    sys.add_node("client1", "client1", 0);
    sys.connect_node_to_network("client1");

    sys.add_process(
        "client1",
        ReplicaNotifiedClientStub::new(
            "client1".to_owned(),
            "123pass321".to_owned(),
            Address::new_ref("server1", 0, "server1"),
            Address::new_ref("server2", 0, "server2"),
        ),
        "client1",
    );

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );
    sys.step_until_no_events();

    let history = read_history(&mut sys, "client1", "client1");
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].kind, ChatEventKind::Created());
    assert_eq!(history[1].kind, ChatEventKind::Connected());
    assert!(
        (history[2].kind == ChatEventKind::SentMessage("hello".to_owned())
            && history[3].kind == ChatEventKind::Connected())
            || (history[3].kind == ChatEventKind::SentMessage("hello".to_owned())
                && history[2].kind == ChatEventKind::Connected())
    );

    sys.rerun_node("server1");
    sys.add_process(
        "server1",
        ServerProcess::new_with_replica(Address::new_ref("server2", 0, "server2")),
        "server1",
    );
    sys.connect_node_to_network("server1");
    sys.send_local_message(
        "server1",
        "server1",
        Message::new("download_events_from_replica", &String::new()).unwrap(),
    );
    sys.step_until_no_events();

    sys.send_local_message("client1", "client1", ClientRequestKind::Status.into());
    sys.step_until_no_events();

    let history = read_history(&mut sys, "client1", "client1");
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].kind, ChatEventKind::Created());
    assert_eq!(history[1].kind, ChatEventKind::Connected());
    assert!(
        (history[2].kind == ChatEventKind::SentMessage("hello".to_owned())
            && history[3].kind == ChatEventKind::Connected())
            || (history[3].kind == ChatEventKind::SentMessage("hello".to_owned())
                && history[2].kind == ChatEventKind::Connected())
    );

    sys.crash_node("server2");
    sys.step_until_no_events();

    sys.recover_node("server2");
    sys.add_process(
        "server2",
        ServerProcess::new_with_replica(Address::new_ref("server1", 0, "server1")),
        "server2",
    );
    sys.connect_node_to_network("server2");
    sys.send_local_message(
        "server2",
        "server2",
        Message::new("download_events_from_replica", &String::new()).unwrap(),
    );
    sys.step_until_no_events();

    sys.crash_node("server1");
    sys.step_until_no_events();

    sys.send_local_message("client1", "client1", ClientRequestKind::Status.into());
    sys.step_until_no_events();

    let history = read_history(&mut sys, "client1", "client1");
    assert_eq!(history.len(), 4);
}
