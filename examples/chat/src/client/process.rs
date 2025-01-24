use dsbuild::{Address, Context, Message, Process};

use crate::server::messages::ServerMessage;

use super::{
    requests::{ClientRequestKind, RequestBuilder},
    state::{State, StateUpdateResult},
};

#[derive(Debug, Clone)]
pub struct ClientProcess {
    server_1_address: Address,
    server_2_address: Option<Address>,
    self_address: Address,
    state_machine: State,
    request_builder: RequestBuilder,
}

impl ClientProcess {
    pub fn new(
        server_address: Address,
        self_address: Address,
        name: String,
        password: String,
    ) -> Self {
        Self {
            server_1_address: server_address,
            server_2_address: None,
            self_address,
            state_machine: State::default(),
            request_builder: RequestBuilder::new(name, password),
        }
    }

    pub fn new_with_replica(
        server_1_address: Address,
        server_2_address: Address,
        self_address: Address,
        name: String,
        password: String,
    ) -> Self {
        Self {
            server_1_address,
            server_2_address: Some(server_2_address),
            self_address,
            state_machine: State::default(),
            request_builder: RequestBuilder::new(name, password),
        }
    }

    fn handle_state_update(&mut self, update: StateUpdateResult, ctx: Context) {
        for info in update.to_user.into_iter() {
            ctx.send_local(info.into())
        }

        if let Some(to_server) = update.to_server {
            let server = self.server_1_address.clone();
            let replica = self.server_2_address.clone();
            let self_address = self.self_address.clone();

            ctx.clone().spawn(async move {
                let request_id = to_server.id;

                let send_result = ctx
                    .send_with_ack(to_server.clone().into(), server, 5.0)
                    .await;

                let success = if send_result.is_err() {
                    if let Some(replica) = replica {
                        ctx.send_with_ack(to_server.clone().into(), replica, 5.0)
                            .await
                            .is_ok()
                    } else {
                        false
                    }
                } else {
                    true
                };

                if !success {
                    let msg = Self::emit_server_response_error(
                        request_id,
                        "can not send request on server".to_string(),
                    );
                    ctx.send(msg.into(), self_address);
                }
            });
        }
    }

    fn emit_server_response_error(request_id: u64, error: String) -> ServerMessage {
        ServerMessage::RequestResponse(request_id, Err(error))
    }
}

impl Process for ClientProcess {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        let request_kind = msg.data::<ClientRequestKind>().unwrap();
        let request = self.request_builder.build_with_kind(request_kind);
        let update_result = self.state_machine.apply_client_request(request);
        self.handle_state_update(update_result, ctx);
    }

    fn on_timer(&mut self, _: String, _: Context) {
        unreachable!("no timers in client")
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) {
        if from != self.server_1_address {
            if let Some(server_2) = &self.server_2_address {
                if from != *server_2 {
                    return;
                }
            } else {
                return;
            }
        }
        let server_msg = msg.data::<ServerMessage>().unwrap();
        let update_result = self.state_machine.apply_server_msg(server_msg);
        self.handle_state_update(update_result, ctx);
    }
}
