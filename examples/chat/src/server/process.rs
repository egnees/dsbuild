use std::sync::Arc;

use dsbuild::{Address, Context, Message, Process};
use tokio::sync::Mutex;

use super::state::{ServerState, ServerStateLock};

#[derive(Default)]
pub struct ServerProcess {
    state_lock: ServerStateLock,
}

impl ServerProcess {
    pub fn new_with_replica(replica: Address) -> Self {
        Self {
            state_lock: Arc::new(Mutex::new(ServerState::new_with_replica(replica))),
        }
    }
}

impl Process for ServerProcess {
    fn on_local_message(&mut self, _msg: Message, ctx: Context) -> Result<(), String> {
        // check replication msg.
        let state_lock = self.state_lock.clone();
        ctx.clone().spawn(async move {
            let mut state_guard = state_lock.lock().await;
            state_guard.check_replication(ctx).await;
        });
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        let state_lock = self.state_lock.clone();
        ctx.clone().spawn(async move {
            let mut state_guard = state_lock.lock().await;
            state_guard.process_msg(from, ctx, msg).await;
        });
        Ok(())
    }
}
