use std::{mem::swap, time::SystemTime};

use rand::{distributions::Alphanumeric, Rng};

use crate::{
    client::requests::{ClientRequest, ClientRequestKind},
    server::{chat::event::ChatEvent, process::messages::ServerMessage},
};

use super::proc::ChatServer;

#[derive(Clone)]
struct ChatClientStub {
    name: String,
    password: String,
    server: dsbuild::Address,
    req_id: usize,
}

impl dsbuild::Process for ChatClientStub {
    fn on_local_message(
        &mut self,
        msg: dsbuild::Message,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        let request_id = self.req_id;
        self.req_id += 1;
        let request_kind = msg.get_data::<ClientRequestKind>().unwrap();
        let request = ClientRequest {
            id: request_id,
            client: self.name.clone(),
            password: self.password.clone(),
            time: SystemTime::now(),
            kind: request_kind,
        };
        let server = self.server.clone();

        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(request.into(), server, 5.0).await;
        });

        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: dsbuild::Context) -> Result<(), String> {
        unreachable!("no timers")
    }

    fn on_message(
        &mut self,
        msg: dsbuild::Message,
        from: dsbuild::Address,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        assert_eq!(from, self.server);
        ctx.send_local(msg);
        Ok(())
    }
}

fn build_system(server: &str, clients: Vec<&str>) -> dsbuild::VirtualSystem {
    let mut system = dsbuild::VirtualSystem::new(12345);

    system.network().set_corrupt_rate(0.0);
    system.network().set_drop_rate(0.0);
    system.network().set_delays(0.1, 0.2);

    system.add_node_with_storage(server, server, 0, 100 * 1024 * 1024 * 1024); // 100 GB.
    system.add_process(server, ChatServer::default(), server);
    system.network().connect_node(server);

    for client in clients.iter() {
        system.add_node(client, client, 0);
        system.add_process(
            client,
            ChatClientStub {
                name: client.to_string(),
                password: "pass".into(),
                server: dsbuild::Address {
                    host: server.to_owned(),
                    port: 0,
                    process_name: server.to_owned(),
                },
                req_id: 0,
            },
            client,
        );
        system.network().connect_node(client);
    }

    system
}

fn get_chat_events(messages: Vec<dsbuild::Message>) -> Vec<ChatEvent> {
    messages
        .into_iter()
        .filter(|m| match m.get_data::<ServerMessage>().unwrap() {
            ServerMessage::RequestResponse(_, _) => false,
            ServerMessage::ChatEvents(_, _) => true,
        })
        .into_iter()
        .map(|m| match m.get_data::<ServerMessage>().unwrap() {
            ServerMessage::RequestResponse(_, _) => panic!("filter out"),
            ServerMessage::ChatEvents(_, events) => {
                assert_eq!(events.len(), 1);
                events[0].clone()
            }
        })
        .collect()
}

#[test]
fn just_works() {
    let mut sys = build_system("server", vec!["client1", "client2"]);

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Create("chat1".into()).into(),
    );

    let responses = sys.step_until_local_message("client1", "client1").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerMessage>().unwrap();
    assert_eq!(response, ServerMessage::RequestResponse(0, Ok(())));

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.send_local_message(
        "client2",
        "client2",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.step_until_no_events();

    let mut client1_events = get_chat_events(sys.read_local_messages("client1", "client1"));
    client1_events.sort();

    let mut client2_events = get_chat_events(sys.read_local_messages("client2", "client2"));
    client1_events.sort();

    if client1_events.len() > client2_events.len() {
        swap(&mut client1_events, &mut client2_events);
    }

    assert_eq!(client1_events.len(), 2);
    assert_eq!(client2_events.len(), 3);

    assert_eq!(client1_events, client2_events.as_slice()[..2]);
}

#[test]
fn chat_history_and_user_passwords_are_persistent() {
    let mut sys = build_system("server", vec!["client1", "client2"]);

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Create("chat1".into()).into(),
    );

    let responses = sys.step_until_local_message("client1", "client1").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerMessage>().unwrap();
    assert_eq!(response, ServerMessage::RequestResponse(0, Ok(())));

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.send_local_message(
        "client2",
        "client2",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.step_until_no_events();

    sys.read_local_messages("client1", "client1");
    sys.read_local_messages("client2", "client2");

    sys.shutdown_node("server");
    sys.step_until_no_events();
    sys.rerun_node("server");

    sys.add_process("server", ChatServer::default(), "server");

    // sys.send_local_message(
    //     "client3",
    //     "client3",
    //     ClientRequestKind::Connect("chat1".into()).into(),
    // );

    // sys.step_until_no_events();

    // let messages = get_chat_events(sys.read_local_messages("client3", "client3"));
    // assert_eq!(messages.len(), 4);

    let cheater = ChatClientStub {
        name: "client1".into(),
        password: "fake_password".into(),
        server: dsbuild::Address {
            host: "server".into(),
            port: 0,
            process_name: "server".into(),
        },
        req_id: 0,
    };

    sys.add_node("cheater", "cheater", 0);
    sys.network().connect_node("cheater");
    sys.add_process("cheater", cheater, "cheater");

    sys.step_until_no_events();

    sys.send_local_message(
        "cheater",
        "cheater",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.step_until_no_events();

    let responses = sys.read_local_messages("cheater", "cheater");
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerMessage>().unwrap();
    match response {
        ServerMessage::RequestResponse(id, res) => {
            assert_eq!(id, 0);
            assert!(res.is_err());
        }
        ServerMessage::ChatEvents(_, _) => panic!("connected to chat with wrong password"),
    };

    sys.send_local_message(
        "client1",
        "client1",
        ClientRequestKind::Connect("chat1".into()).into(),
    );

    sys.step_until_no_events();

    let chat_events = get_chat_events(sys.read_local_messages("client1", "client1"));
    assert_eq!(chat_events.len(), 4);
}

#[test]
fn stress() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let chats = vec!["chat1", "chat2", "chat3"];
    let clients = vec![
        "client1", "client2", "client3", "client4", "client5", "client6",
    ];

    assert!(clients.len() % chats.len() == 0);

    let mut sys = build_system("server", clients.clone());

    for chat in chats.iter() {
        sys.send_local_message(
            "client1",
            "client1",
            ClientRequestKind::Create(chat.to_string()).into(),
        );
    }

    sys.step_until_no_events();

    const ITERS: usize = 10;
    for iter in 0..ITERS {
        for i in 0..clients.len() {
            let chat = (i + iter) % chats.len();
            sys.send_local_message(
                clients[i],
                clients[i],
                ClientRequestKind::Connect(chats[chat].into()).into(),
            );
        }

        sys.step_until_no_events();

        for i in 0..clients.len() {
            // let msg: String = rand::thread_rng()
            //     .sample_iter(&Alphanumeric)
            //     .take(16 * 1024 + rand::thread_rng().gen_range(0..512)) // ~16Kb
            //     .map(char::from)
            //     .collect();

            sys.send_local_message(
                clients[i],
                clients[i],
                ClientRequestKind::SendMessage("Hello".into()).into(),
            );
        }

        sys.step_until_no_events();

        for i in 0..clients.len() {
            sys.send_local_message(clients[i], clients[i], ClientRequestKind::Disconnect.into());
        }

        sys.step_until_no_events();
    }

    sys.read_local_messages("client1", "client1");

    for chat in chats.iter() {
        sys.send_local_message(
            "client1",
            "client1",
            ClientRequestKind::Connect(chat.to_string()).into(),
        );

        sys.step_until_no_events();

        let events = get_chat_events(sys.read_local_messages("client1", "client1"));
        println!("{}", events.len());

        sys.send_local_message("client1", "client1", ClientRequestKind::Disconnect.into());

        sys.step_until_no_events();
    }
}
