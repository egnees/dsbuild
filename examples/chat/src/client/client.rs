use dsbuild::{Address, Context, Message, Process};

use crate::server::messages::ServerMessage;

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

                let send_result = ctx
                    .send_with_ack(to_server.into(), server.clone(), 5.0)
                    .await;

                if let Err(err) = send_result {
                    let msg = Self::emit_server_response_error(
                        request_id,
                        format!("can not send request on server: {:?}", err),
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
