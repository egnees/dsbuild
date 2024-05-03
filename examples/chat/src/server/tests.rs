use std::{collections::BTreeSet, time::SystemTime};

use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};
use rand_pcg::Pcg64;
use rand_seeder::Seeder;

use dsbuild::{Address, Context, Message, Process, VirtualSystem};

use crate::{
    client::requests::{ClientRequest, ClientRequestKind},
    server::{event::ChatEvent, messages::ServerMessage},
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
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let kind = msg.get_data::<ClientRequestKind>().unwrap();
        let req = ClientRequest {
            id: self.req_id,
            client: self.login.clone(),
            password: self.password.clone(),
            time: SystemTime::now(),
            kind,
        };
        self.req_id += 1;
        let to = self.server.clone();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(req.into(), to, 5.0).await;
        });
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        assert_eq!(from, self.server);
        ctx.send_local(msg);
        Ok(())
    }
}

#[test]
fn state_works() {
    // env_logger::Builder::new()
    //     .filter_level(log::LevelFilter::Debug)
    //     .init();

    let mut sys = VirtualSystem::new(12345);
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

    let messages = sys
        .read_local_messages("client", "client")
        .unwrap()
        .into_iter()
        .map(|msg| msg.get_data::<ServerMessage>().unwrap())
        .filter(|msg| match msg {
            ServerMessage::RequestResponse(id, res) => {
                assert_eq!(*id, 1);
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

    let chat_history = sys
        .read_local_messages("client", "client")
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

    let new_events = sys
        .read_local_messages("client", "client")
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

    assert_eq!(new_events.len(), 3);

    sys.shutdown_node("server");

    sys.step_until_no_events();

    sys.rerun_node("server");
    sys.add_process("server", ServerProcess::default(), "server");

    sys.send_local_message(
        "client",
        "client",
        ClientRequestKind::Connect("chat".to_string()).into(),
    );

    sys.step_until_no_events();

    let history = sys
        .read_local_messages("client", "client")
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

    assert_eq!(history.len(), 9);
    for (i, event) in history.iter().enumerate() {
        assert_eq!(i as u64, event.seq);
    }
}

#[test]
fn state_multiple_users() {
    // env_logger::Builder::new()
    //     .filter_level(log::LevelFilter::Debug)
    //     .init();

    let mut chats = vec!["chat1", "chat2", "chat3", "chat4", "chat5"];
    let clients = vec!["client1", "client2", "client3", "client4", "client5"];

    let mut sys = VirtualSystem::new(12345);
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

    sys.network().set_delays(0.5, 1.0);
    sys.network().set_drop_rate(0.05);

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
