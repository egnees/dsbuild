use std::time::SystemTime;

use dsbuild::{Address, Context, Message, Process};

use crate::server::messages::{ServerMessage, ServerMessageKind};

use super::{
    requests::{ClientRequest, ClientRequestKind},
    state::{State, StateUpdateResult},
};

#[derive(Debug, Clone)]
struct Client {
    server_address: Address,
    self_address: Address,
    name: String,
    password: String,
    state_machine: State,
    last_request_id: usize,
}

impl Client {
    fn next_request_id(&mut self) -> usize {
        self.last_request_id += 1;
        self.last_request_id
    }

    fn make_client_request(&mut self, client_request_kind: ClientRequestKind) -> ClientRequest {
        ClientRequest {
            id: self.next_request_id(),
            client: self.name.clone(),
            password: self.password.clone(),
            time: SystemTime::now(),
            kind: client_request_kind,
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
        let request = self.make_client_request(request_kind);
        let update_result = self.state_machine.apply_client_request(request);
        self.handle_state_update(update_result, ctx);
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        unreachable!("No timers in client.")
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
