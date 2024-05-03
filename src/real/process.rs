//! Definition of process management objects.

use std::sync::{Arc, Mutex, RwLock};

use log::warn;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    common::{context::Context, message::RoutedMessage},
    Address, Message, Process,
};

use super::{context::RealContext, network::NetworkRequest, timer::TimerManager};

/// All messages which can be received from system.
pub enum FromSystemMessage {
    NetworkMessage(RoutedMessage),
    Suspend(),
}

/// All messages which can be sent to system.
pub enum ToSystemMessage {
    ProcessStopped(String), // Process name.
}

/// Proxy between system and process implementation.
/// Calls process methods, when receives network message, timer fires,
/// or local message appears.
pub(crate) struct ProcessManager {
    /// Waiters.
    local_receiver: Receiver<Message>,
    system_receiver: Receiver<FromSystemMessage>,
    timers_receiver: Receiver<String>,
    /// To communicate with outside.
    /// Must be passed to real context.
    output: Output,
    /// Address of the process.
    /// Used to communicate in network.
    address: Address,
    /// Process implementation, provided by user.
    process: Arc<RwLock<dyn Process>>,
    mount_dir: String,
}

/// Responsible for process communication with outside.
/// Used to communicate in network.
#[derive(Clone)]
pub(crate) struct Output {
    pub local: Sender<Message>,
    pub network: Sender<NetworkRequest>,
    pub system: Sender<ToSystemMessage>,
    pub timer_mngr: Arc<Mutex<TimerManager>>,
}

pub(crate) struct ProcessManagerConfig {
    pub address: Address,
    pub process: Arc<RwLock<dyn Process>>,
    pub local_sender: Sender<Message>,
    pub local_receiver: Receiver<Message>,
    pub system_sender: Sender<ToSystemMessage>,
    pub system_receiver: Receiver<FromSystemMessage>,
    pub network_sender: Sender<NetworkRequest>,
    pub max_buffer_size: usize,
    pub mount_dir: String,
}

impl ProcessManager {
    /// Create new process manager.
    pub fn new(config: ProcessManagerConfig) -> Self {
        let (timer_sender, timers_receiver) = channel(config.max_buffer_size);

        let timer_manager = TimerManager::new(timer_sender);
        let timer_manager_ref = Arc::new(Mutex::new(timer_manager));

        let output = Output {
            local: config.local_sender,
            network: config.network_sender,
            system: config.system_sender,
            timer_mngr: timer_manager_ref,
        };

        Self {
            local_receiver: config.local_receiver,
            system_receiver: config.system_receiver,
            timers_receiver,
            output,
            address: config.address,
            process: config.process,
            mount_dir: config.mount_dir,
        }
    }

    /// Run cycle of the process manager.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.local_receiver.recv() => self.handle_local_message(msg),
                Some(msg) = self.system_receiver.recv() => {
                    match msg {
                        FromSystemMessage::NetworkMessage(msg) => self.handle_message(msg),
                        FromSystemMessage::Suspend() => break
                    }
                },
                Some(timer_name) = self.timers_receiver.recv() => self.handle_timer_fired(timer_name),
                else => break
            }
        }
    }

    fn create_context(&self) -> Context {
        let real = RealContext {
            output: self.output.clone(),
            address: self.address.clone(),
            mount_dir: self.mount_dir.clone(),
        };
        Context::new_real(real)
    }

    fn handle_local_message(&mut self, msg: Message) {
        let result = self
            .process
            .write()
            .unwrap()
            .on_local_message(msg.clone(), self.create_context());

        if let Err(info) = result {
            warn!(
                "Error on process {}: handle local message {:?}; error: {}",
                self.address.process_name, msg, info
            );
        }
    }

    fn handle_message(&mut self, msg: RoutedMessage) {
        let result = self.process.write().unwrap().on_message(
            msg.msg.clone(),
            msg.from.clone(),
            self.create_context(),
        );

        if let Err(info) = result {
            warn!(
                "Error on process {}: handle message {:?} from {:?}; error: {}",
                self.address.process_name, msg.msg, msg.from, info
            );
        }
    }

    fn handle_timer_fired(&mut self, timer_name: String) {
        let result = self
            .process
            .write()
            .unwrap()
            .on_timer(timer_name.clone(), self.create_context());

        if let Err(info) = result {
            warn!(
                "Error on process {}: handle timer {:?}; error: {}",
                self.address.process_name, timer_name, info
            );
        }
    }
}
