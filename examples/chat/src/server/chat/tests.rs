use dsbuild;
use serde::{Deserialize, Serialize};

use rand::{distributions::Alphanumeric, Rng}; // 0.8

use crate::server::process::messages::ServerMessage;

use super::{
    event::{ChatEvent, ChatEventKind},
    handler::RequestHandler,
    manager::ChatsManager,
};

#[derive(Clone)]
struct ChatClientStub {
    server: dsbuild::Address,
}

#[derive(Serialize, Deserialize)]
struct ClientRequestStub {
    client_name: String,
    chat_name: String,
    event_kind: ChatEventKind,
}

impl ClientRequestStub {
    pub fn new(client_name: &str, chat_name: &str, event_kind: ChatEventKind) -> Self {
        Self {
            client_name: client_name.to_string(),
            chat_name: chat_name.to_string(),
            event_kind,
        }
    }
}

impl dsbuild::Process for ChatClientStub {
    fn on_local_message(
        &mut self,
        msg: dsbuild::Message,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        ctx.send(msg, self.server.clone());
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: dsbuild::Context) -> Result<(), String> {
        unreachable!("no timers")
    }

    fn on_message(
        &mut self,
        msg: dsbuild::Message,
        _from: dsbuild::Address,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        ctx.send_local(msg);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct ServerStub {
    manager: ChatsManager,
}

#[derive(Serialize, Deserialize)]
struct ServerBroadcast {
    broadcast_participants: Vec<String>,
    chat_event: ChatEvent,
}

impl dsbuild::Process for ServerStub {
    fn on_local_message(
        &mut self,
        _msg: dsbuild::Message,
        _ctx: dsbuild::Context,
    ) -> Result<(), String> {
        unreachable!("no local messages")
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
        let client_request = msg.get_data::<ClientRequestStub>().unwrap();

        let chat_locker = self.manager.get_chat_lock(&client_request.chat_name);

        ctx.clone().spawn(async move {
            let mut chat_guard = chat_locker.lock().await;

            if !chat_guard.is_initialized {
                chat_guard.init(ctx.clone()).await;
            }

            let request_handler = RequestHandler::new(
                chat_guard,
                client_request.event_kind,
                client_request.client_name,
                from,
                ctx.clone(),
            );

            if let Some((chat_event, broadcast_participants)) = request_handler.handle().await {
                ctx.send_local(
                    dsbuild::Message::borrow_new(
                        "server_broadcast",
                        ServerBroadcast {
                            broadcast_participants,
                            chat_event,
                        },
                    )
                    .unwrap(),
                )
            }
        });
        Ok(())
    }
}

fn build_system(server: &str, clients: Vec<&str>) -> dsbuild::VirtualSystem {
    let mut system = dsbuild::VirtualSystem::new(12345);

    system.network().set_corrupt_rate(0.0);
    system.network().set_drop_rate(0.0);
    system.network().set_delays(0.1, 0.2);

    system.add_node_with_storage(server, server, 0, 100 * 1024 * 1024 * 1024); // 100 GB.
    system.add_process(server, ServerStub::default(), server);
    system.network().connect_node(server);

    for client in clients.iter() {
        system.add_node(client, client, 0);
        system.add_process(
            client,
            ChatClientStub {
                server: dsbuild::Address {
                    host: server.to_owned(),
                    port: 0,
                    process_name: server.to_owned(),
                },
            },
            client,
        );
        system.network().connect_node(client);
    }

    system
}

#[test]
fn just_works() {
    let mut sys = build_system("server", vec!["client"]);

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Created()),
        )
        .unwrap(),
    );
    let events = sys.step_until_local_message("server", "server").unwrap();
    assert_eq!(events.len(), 1);
    let broadcast = events[0].get_data::<ServerBroadcast>().unwrap();
    assert_eq!(broadcast.chat_event.seq, 0);
    assert_eq!(broadcast.chat_event.chat, "test_chat");
    assert!(broadcast.broadcast_participants.is_empty());

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Connected()),
        )
        .unwrap(),
    );
    let events = sys.step_until_local_message("server", "server").unwrap();
    assert_eq!(events.len(), 1);
    let broadcast = events[0].get_data::<ServerBroadcast>().unwrap();
    assert_eq!(broadcast.chat_event.seq, 1);
    assert_eq!(broadcast.chat_event.chat, "test_chat");
    assert_eq!(broadcast.broadcast_participants, vec!["client"]);
    sys.step_until_no_events();
    let client_events = sys.read_local_messages("client", "client");
    assert_eq!(client_events.len(), 2);
    let mut events: Vec<_> = client_events
        .into_iter()
        .map(|m| {
            let server_message = m.get_data::<ServerMessage>().unwrap();
            match server_message {
                ServerMessage::RequestResponse(_, _) => panic!("no responses on requests"),
                ServerMessage::ChatEvents(chat_name, events) => {
                    assert_eq!(chat_name, "test_chat");
                    assert_eq!(events.len(), 1);
                    events[0].clone()
                }
            }
        })
        .collect();
    events.sort(); // sort by seq.
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].chat, "test_chat");
    assert_eq!(events[0].seq, 0);
    assert_eq!(events[0].kind, ChatEventKind::Created());
    assert_eq!(events[0].user, "client");
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].chat, "test_chat");
    assert_eq!(events[1].seq, 1);
    assert_eq!(events[1].kind, ChatEventKind::Connected());
    assert_eq!(events[1].user, "client");

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Disconnected()),
        )
        .unwrap(),
    );
    let events = sys.step_until_local_message("server", "server").unwrap();
    assert_eq!(events.len(), 1);
    let broadcast = events[0].get_data::<ServerBroadcast>().unwrap();
    assert_eq!(broadcast.chat_event.seq, 2);
    assert_eq!(broadcast.chat_event.chat, "test_chat");
    assert!(broadcast.broadcast_participants.is_empty());
}

#[test]
fn multiple_users_multiple_chats_concurrent() {
    let chats = vec!["chat1", "chat2", "chat3", "chat4", "chat5"];
    let clients = vec!["client1", "client2", "client3", "client4", "client5"];

    let mut sys = build_system("server", clients.clone());

    for i in 0..5 {
        let client = clients[i];
        let chat = chats[i];
        sys.send_local_message(
            client,
            client,
            dsbuild::Message::borrow_new(
                "client_request",
                ClientRequestStub::new(client, chat, ChatEventKind::Created()),
            )
            .unwrap(),
        );
    }

    sys.step_until_no_events();

    for client in clients.iter() {
        for chat in chats.iter() {
            sys.send_local_message(
                client,
                client,
                dsbuild::Message::borrow_new(
                    "client_request",
                    ClientRequestStub::new(client, chat, ChatEventKind::Connected()),
                )
                .unwrap(),
            );
        }
    }

    sys.step_until_no_events();

    const ITERS: usize = 5;

    for _ in 0..ITERS {
        for client in clients.iter() {
            for chat in chats.iter() {
                let msg: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16 * 1024 + 123) // 16Kb
                    .map(char::from)
                    .collect();

                sys.send_local_message(
                    client,
                    client,
                    dsbuild::Message::borrow_new(
                        "client_request",
                        ClientRequestStub::new(client, chat, ChatEventKind::SentMessage(msg)),
                    )
                    .unwrap(),
                );
            }
        }

        sys.step_until_no_events();
    }

    for client in clients.iter() {
        let _ = sys.read_local_messages(client, client);
    }

    let total_chat_events: usize = clients.len() * (1 + ITERS) + 1;
    let client = clients[0];
    for chat in chats.iter() {
        sys.send_local_message(
            client,
            client,
            dsbuild::Message::borrow_new(
                "client_request",
                ClientRequestStub::new(client, chat, ChatEventKind::Connected()),
            )
            .unwrap(),
        );

        sys.step_until_no_events();

        let history = sys.read_local_messages(client, client);
        assert_eq!(history.len(), total_chat_events + 1); // +1 for recent connect.
    }
}

#[test]
fn persistent_chats_history() {
    let mut sys = build_system("server", vec!["client"]);

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Created()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Connected()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    let mut chat_history_until_server_restart: Vec<_> = sys
        .read_local_messages("client", "client")
        .into_iter()
        .map(|m| {
            let server_message = m.get_data::<ServerMessage>().unwrap();
            match server_message {
                ServerMessage::RequestResponse(_, _) => panic!("no responses on requests"),
                ServerMessage::ChatEvents(chat_name, events) => {
                    assert_eq!(chat_name, "test_chat");
                    assert_eq!(events.len(), 1);
                    events[0].clone()
                }
            }
        })
        .collect();
    assert_eq!(chat_history_until_server_restart.len(), 2);
    chat_history_until_server_restart.sort();

    sys.shutdown_node("server");

    sys.step_until_no_events();

    sys.rerun_node("server");

    sys.add_process("server", ServerStub::default(), "server");

    sys.step_until_no_events();

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Connected()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    let mut chat_history_after_server_restart: Vec<_> = sys
        .read_local_messages("client", "client")
        .into_iter()
        .map(|m| {
            let server_message = m.get_data::<ServerMessage>().unwrap();
            match server_message {
                ServerMessage::RequestResponse(_, _) => panic!("no responses on requests"),
                ServerMessage::ChatEvents(chat_name, events) => {
                    assert_eq!(chat_name, "test_chat");
                    assert_eq!(events.len(), 1);
                    events[0].clone()
                }
            }
        })
        .collect();

    chat_history_after_server_restart.sort();
    assert_eq!(chat_history_after_server_restart.len(), 3);

    assert_eq!(
        chat_history_until_server_restart,
        chat_history_after_server_restart.as_slice()[..2]
    );
}

#[test]
#[should_panic]
fn crash_tolerant() {
    let mut sys = build_system("server", vec!["client"]);

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Created()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Connected()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    let mut chat_history_until_server_restart: Vec<_> = sys
        .read_local_messages("client", "client")
        .into_iter()
        .map(|m| {
            let server_message = m.get_data::<ServerMessage>().unwrap();
            match server_message {
                ServerMessage::RequestResponse(_, _) => panic!("no responses on requests"),
                ServerMessage::ChatEvents(chat_name, events) => {
                    assert_eq!(chat_name, "test_chat");
                    assert_eq!(events.len(), 1);
                    events[0].clone()
                }
            }
        })
        .collect();
    assert_eq!(chat_history_until_server_restart.len(), 2);
    chat_history_until_server_restart.sort();

    sys.crash_node("server");

    sys.step_until_no_events();

    sys.recover_node("server");

    sys.add_process("server", ServerStub::default(), "server");

    sys.step_until_no_events();

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new("client", "test_chat", ChatEventKind::Connected()),
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    let mut chat_history_after_server_restart: Vec<_> = sys
        .read_local_messages("client", "client")
        .into_iter()
        .map(|m| {
            let server_message = m.get_data::<ServerMessage>().unwrap();
            match server_message {
                ServerMessage::RequestResponse(_, _) => panic!("no responses on requests"),
                ServerMessage::ChatEvents(chat_name, events) => {
                    assert_eq!(chat_name, "test_chat");
                    assert_eq!(events.len(), 1);
                    events[0].clone()
                }
            }
        })
        .collect();

    chat_history_after_server_restart.sort();
    assert_eq!(chat_history_after_server_restart.len(), 3);

    assert_eq!(
        chat_history_until_server_restart,
        chat_history_after_server_restart.as_slice()[..2]
    );
}
