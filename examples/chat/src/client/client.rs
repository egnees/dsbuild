use std::time::SystemTime;

use dsbuild::{Address, Context, Message, Process};

use crate::server::messages::{ServerMessage, ServerMessageKind};

use super::{
    requests::{ClientRequestKind, RequestBuilder},
    state::{State, StateUpdateResult},
};

#[derive(Debug, Clone)]
pub struct Client {
    server_address: Address,
    self_address: Address,
    state_machine: State,
    request_builder: RequestBuilder,
}

impl Client {
    pub fn new(
        server_address: Address,
        self_address: Address,
        name: String,
        password: String,
    ) -> Self {
        Self {
            server_address,
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
            let server = self.server_address.clone();
            let self_address = self.self_address.clone();

            ctx.clone().spawn(async move {
                let request_id = to_server.id;
                let server_name = server.process_name.clone();

                let send_result = ctx.send_reliable(to_server.into(), server.clone()).await;

                if let Err(info) = send_result {
                    let msg = Self::emit_server_response_error(
                        request_id,
                        server_name,
                        format!("can not send request on server: {}", info),
                    );

                    ctx.send(msg.into(), self_address);
                }
            });
        }
    }

    fn emit_server_response_error(
        request_id: usize,
        server: String,
        error: String,
    ) -> ServerMessage {
        ServerMessage {
            server,
            time: SystemTime::now(),
            kind: ServerMessageKind::RequestResponse(request_id, Err(error)),
        }
    }
}

impl Process for Client {
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let request_kind = msg.get_data::<ClientRequestKind>().unwrap();
        let request = self.request_builder.build_with_kind(request_kind);
        let update_result = self.state_machine.apply_client_request(request);
        self.handle_state_update(update_result, ctx);
        Ok(())
    }

    fn on_timer(&mut self, _: String, _: Context) -> Result<(), String> {
        unreachable!("no timers in client")
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        if from != self.server_address {
            return Ok(());
        }
        let server_msg = msg.get_data::<ServerMessage>().unwrap();
        let update_result = self.state_machine.apply_server_msg(server_msg);
        self.handle_state_update(update_result, ctx);
        Ok(())
    }
}
