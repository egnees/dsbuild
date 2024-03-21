//! Definition of chat client process.

use dsbuild::{Address, Context, Message, Process};

/// Represents a chat client process.
#[derive(Clone)]
pub struct Client {
    name: String,
    server_addr: Address,
}

impl Client {
    pub fn new(name: &str, server_addr: Address) -> Self {
        Client {
            name: name.to_string(),
            server_addr,
        }
    }
}

impl Process for Client {
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        todo!()
    }

    fn on_timer(&mut self, name: String, ctx: Context) -> Result<(), String> {
        todo!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        todo!()
    }
}
