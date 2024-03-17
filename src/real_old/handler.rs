//! Definition of process actions handler.

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use crate::{Address, Message};

use super::{
    network::{grpc_messenger::GRpcMessenger, network_manager::NetworkManager},
    time::{basic_timer_setter::BasicTimerSetter, time_manager::TimeManager},
};

/// Represents thread safe process actions handler.
pub struct Handler {
    /// Network manager.
    network_manager: NetworkManager<GRpcMessenger>,
    /// Time manager.
    time_manager: TimeManager<BasicTimerSetter>,
    /// Local messages sender.
    sender: Sender<Message>,
}

/// Use handler ref to access to handler.
pub type HandlerRef = Arc<Handler>;

impl Handler {
    /// Create new handler.
    pub fn new(
        network_manager: NetworkManager<GRpcMessenger>,
        time_manager: TimeManager<BasicTimerSetter>,
        sender: Sender<Message>,
    ) -> Self {
        Self {
            network_manager,
            time_manager,
        }
    }

    pub fn send(&self, from: Address, to: Address, msg: Message) {
        self.network_manager.send_message(from, to, msg)
    }

    pub fn send_local_message(&self, proc: &str, node: &str, msg: Message) {
        self.network_manager.send_local_message(proc, node, msg)
    }
}
