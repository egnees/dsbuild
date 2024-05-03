//! Definition of [`dsbuild::Process`] for chat server.

use std::sync::Arc;

use dsbuild::{Address, Context, Message, Process};
use tokio::sync::Mutex;

use crate::{
    client::requests::ClientRequest,
    server::{
        chat::{self, manager::ChatsManager},
        user::{self, manager::UsersManager},
    },
};

use super::messages::{DirectedServerMessage, ServerMessage};

/// Accepts requests in the form of [`ClientRequest`] and answers with [`ServerMessage::RequestResponse`].
/// Send messages to users with chat events in the form of [`ServerMessage::`].
#[derive(Clone, Default)]
pub struct ChatServer {
    chats_manager: Arc<Mutex<ChatsManager>>,
    users_manager: Arc<Mutex<UsersManager>>,
}

impl Process for ChatServer {
    fn on_local_message(&mut self, _msg: Message, _ctx: Context) -> Result<(), String> {
        panic!("no local messages");
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        panic!("no timers");
    }

    async fn on_message(
        &mut self,
        msg: Message,
        from: Address,
        ctx: Context,
    ) -> Result<(), String> {
        let client_request = msg.get_data::<ClientRequest>()?;
        let chats_manager = self.chats_manager.clone();
        let users_manager = self.users_manager.clone();

        ctx.clone().spawn(async move {
            let mut users_manager_guard = users_manager.lock().await;

            let user_lock = users_manager_guard.get_user_lock(&client_request.client, &from);
            drop(users_manager_guard);

            let mut user_state = user_lock.lock().await;

            let init_result = user_state
                .init(ctx.clone(), &client_request.password, &from)
                .await;

            if !init_result {
                let _ = ctx
                    .send_with_ack(
                        Message::borrow_new(
                            "server_message",
                            ServerMessage::RequestResponse(
                                client_request.id,
                                Err("bad password".into()),
                            ),
                        )
                        .unwrap(),
                        from,
                        5.0,
                    )
                    .await;
                return;
            }

            let handler_for_user = user::handler::RequestHandler {
                user_state: &user_state,
                request: client_request.kind,
            };

            let messages: Vec<DirectedServerMessage> = match handler_for_user.handle().await {
                Ok(chat_event) => {
                    user_state.update(chat_event.clone());

                    println!("point 1");

                    let mut chats_managers_guard = chats_manager.lock().await;

                    println!("point 2");

                    let chat_lock = chats_managers_guard.get_chat_lock(&chat_event.chat);
                    // drop(chats_managers_guard);

                    let mut chat_state = chat_lock.lock().await;

                    chat_state.init(ctx.clone()).await;

                    let handler_for_chat = chat::handler::RequestHandler {
                        chat_guard: chat_state,
                        event_kind: chat_event.kind,
                        producer: client_request.client.clone(),
                        address: user_state.addr.clone(),
                        ctx: ctx.clone(),
                    };

                    let handle_result = handler_for_chat.handle().await;

                    if let Some((chat_event, broadcast_participants)) = handle_result {
                        let mut messages = vec![DirectedServerMessage {
                            msg: ServerMessage::RequestResponse(client_request.id, Ok(())),
                            to: user_state.addr.clone(),
                        }];
                        for client in broadcast_participants.iter() {
                            let users_manager_guard = users_manager.lock().await;
                            let user_lock = users_manager_guard
                                .get_user_lock_without_creating(client)
                                .unwrap();
                            drop(users_manager_guard);
                            let user_guard = user_lock.lock().await;
                            let addr = user_guard.addr.clone();
                            let msg = DirectedServerMessage {
                                msg: ServerMessage::ChatEvents(
                                    chat_event.chat.clone(),
                                    vec![chat_event.clone()],
                                ),
                                to: addr,
                            };
                            messages.push(msg);
                        }
                        messages
                    } else {
                        vec![DirectedServerMessage {
                            msg: ServerMessage::RequestResponse(
                                client_request.id,
                                Err("incorrect request, maybe chat with such name not exists"
                                    .into()),
                            ),
                            to: from,
                        }]
                    }
                }
                Err(info) => {
                    vec![DirectedServerMessage {
                        msg: ServerMessage::RequestResponse(client_request.id, Err(info)),
                        to: from,
                    }]
                }
            };

            for message in messages {
                let ctx_clone = ctx.clone();
                ctx.clone().spawn(async move {
                    let _ = ctx_clone
                        .send_with_ack(message.msg.into(), message.to, 5.0)
                        .await;
                });
            }
        });

        Ok(())
    }
}
