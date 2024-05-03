use dsbuild::{Address, Context, Message, Process};

use crate::client::requests::ClientRequest;

use super::state::ServerStateLock;

#[derive(Default)]
pub struct ServerProcess {
    state_lock: ServerStateLock,
}

impl Process for ServerProcess {
    fn on_local_message(&mut self, _msg: Message, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        let client_request = msg.get_data::<ClientRequest>()?;
        let state_lock = self.state_lock.clone();
        ctx.clone().spawn(async move {
            let mut state_guard = state_lock.lock().await;
            state_guard
                .handle_user_request(from, ctx, client_request)
                .await;
        });
        Ok(())
    }
}
