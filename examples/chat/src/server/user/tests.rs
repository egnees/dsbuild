use crate::{client::requests::ClientRequestKind, server::chat::event::ChatEventKind};

use super::{
    handler::{RelatedChatEventKind, RequestHandler},
    manager::UsersManager,
};

use dsbuild;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ClientRequestStub {
    name: String,
    pass: String,
    kind: ClientRequestKind,
}

impl ClientRequestStub {
    pub fn new(name: &str, pass: &str, kind: ClientRequestKind) -> Self {
        Self {
            name: name.to_owned(),
            pass: pass.to_owned(),
            kind,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ServerResponseStub {
    response: Result<RelatedChatEventKind, String>,
}

#[derive(Clone, Default)]
struct ServerStub {
    manager: UsersManager,
}

impl dsbuild::Process for ServerStub {
    fn on_local_message(
        &mut self,
        _msg: dsbuild::Message,
        _ctx: dsbuild::Context,
    ) -> Result<(), String> {
        unimplemented!()
    }

    fn on_timer(&mut self, _name: String, _ctx: dsbuild::Context) -> Result<(), String> {
        unimplemented!()
    }

    fn on_message(
        &mut self,
        msg: dsbuild::Message,
        from: dsbuild::Address,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        let request = msg.get_data::<ClientRequestStub>().unwrap();
        let lock = self.manager.get_user_lock(&request.name, &from);
        ctx.clone().spawn(async move {
            let mut state = lock.lock().await;
            let ok = state.init(ctx.clone(), &request.pass, &from).await;
            if !ok {
                let response = ServerResponseStub {
                    response: Err("bad password".into()),
                };
                let _ = ctx
                    .send_with_ack(
                        dsbuild::Message::borrow_new("server_response", response).unwrap(),
                        state.addr.clone(),
                        5.0,
                    )
                    .await;
            } else {
                let handler = RequestHandler {
                    user_state: &state,
                    request: request.kind,
                };

                let response = match handler.handle().await {
                    Ok(event) => {
                        state.update(event.clone());
                        ServerResponseStub {
                            response: Ok(event),
                        }
                    }
                    Err(e) => ServerResponseStub { response: Err(e) },
                };

                let _ = ctx
                    .send_with_ack(
                        dsbuild::Message::borrow_new("server_response", response).unwrap(),
                        state.addr.clone(),
                        5.0,
                    )
                    .await;
            }
        });

        Ok(())
    }
}

#[derive(Clone)]
struct ClientStub {
    pub server: dsbuild::Address,
}

impl dsbuild::Process for ClientStub {
    fn on_local_message(
        &mut self,
        msg: dsbuild::Message,
        ctx: dsbuild::Context,
    ) -> Result<(), String> {
        let server = self.server.clone();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(msg, server, 5.0).await;
        });
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: dsbuild::Context) -> Result<(), String> {
        unimplemented!()
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
    system.add_process(server, ServerStub::default(), server);
    system.network().connect_node(server);

    for client in clients.iter() {
        system.add_node(client, client, 0);
        system.add_process(
            client,
            ClientStub {
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
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::Connect("unreal_chat".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert_eq!(
        response.response,
        Ok(RelatedChatEventKind {
            kind: ChatEventKind::Connected(),
            chat: "unreal_chat".into()
        })
    );

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::SendMessage("msg".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert_eq!(
        response.response,
        Ok(RelatedChatEventKind {
            kind: ChatEventKind::SentMessage("msg".into()),
            chat: "unreal_chat".into()
        })
    );
}

#[test]
fn passwords_persistent() {
    let mut sys = build_system("server", vec!["client"]);

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::Connect("unreal_chat".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert_eq!(
        response.response,
        Ok(RelatedChatEventKind {
            kind: ChatEventKind::Connected(),
            chat: "unreal_chat".into()
        })
    );

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "1234",
                ClientRequestKind::SendMessage("msg".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert!(response.response.is_err());

    sys.shutdown_node("server");

    sys.step_until_no_events();

    sys.rerun_node("server");
    sys.add_process("server", ServerStub::default(), "server");

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "1234",
                ClientRequestKind::Connect("unreal_chat".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert!(response.response.is_err());

    sys.send_local_message(
        "client",
        "client",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::Connect("unreal_chat".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client", "client").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert!(response.response.is_ok());
}

#[test]
fn user_address_updates() {
    let mut sys = build_system("server", vec!["client1", "client2"]);

    sys.send_local_message(
        "client1",
        "client1",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::Connect("unreal_chat".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client1", "client1").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert_eq!(
        response.response,
        Ok(RelatedChatEventKind {
            kind: ChatEventKind::Connected(),
            chat: "unreal_chat".into()
        })
    );

    sys.send_local_message(
        "client2",
        "client2",
        dsbuild::Message::borrow_new(
            "client_request",
            ClientRequestStub::new(
                "client",
                "123",
                ClientRequestKind::SendMessage("msg".into()),
            ),
        )
        .unwrap(),
    );
    let responses = sys.step_until_local_message("client2", "client2").unwrap();
    assert_eq!(responses.len(), 1);
    let response = responses[0].get_data::<ServerResponseStub>().unwrap();
    assert_eq!(
        response.response,
        Ok(RelatedChatEventKind {
            kind: ChatEventKind::SentMessage("msg".into()),
            chat: "unreal_chat".into()
        })
    );

    let responses = sys.read_local_messages("client1", "client1");
    assert_eq!(responses.len(), 0);
}
