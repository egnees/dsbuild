use dsbuild::{Address, Context, Message, Process};
use serde::{Deserialize, Serialize};

/// Define message types
#[derive(Clone, Serialize)]
pub struct InitiatePingRequest {
    pub receiver: Address,
}

pub struct Ping {}

pub struct Pong {}

/// Define PingPong process
pub struct PingPongProcess {
    pub pings: usize,
    pub pongs: usize,
}

impl Process for PingPongProcess {}
