//! Definition of node in real mode.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicU32, Arc, RwLock},
};

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{common::message::RoutedMessage, Address, Process, ProcessWrapper};

use super::{
    io::IOProcessWrapper,
    network::{self, NetworkRequest},
    process::{FromSystemMessage, ProcessManager, ProcessManagerConfig, ToSystemMessage},
};

////////////////////////////////////////////////////////////////////////////////

/// Represents node in real mode.
///
/// Node is intended to run user-defined [processes][crate::Process].
/// Node manages network, time and file system. The whole network communication of processes,
/// spawned on node, is managed by node. It particular, [network address][crate::Address] of
/// every process is constructed from node listen host, port and process name.
///
/// To [construct][Node::new] node, user must specify it's listen host, port and directory of
/// file system, in which processes will manipulate with files.
///
/// Also user can [spawn][Node::spawn] asynchronous activities on node, which will be executed
/// together will user-defined processes.
///
/// After all processes and activities are spawned, user can [run][Node::run] it.
pub struct Node {
    scheduled: Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    process_senders: HashMap<String, Sender<FromSystemMessage>>,
    from_process_receiver: Receiver<ToSystemMessage>,
    to_system_sender: Sender<ToSystemMessage>, // Just to clone it and pass to different process managers.
    network_sender: Sender<NetworkRequest>,
    network_receiver: Receiver<RoutedMessage>,
    max_buffer_size: usize,
    host: String,
    port: u16,
    mount_dir: String,
}

impl Node {
    /// Allows to create new node with specified listen host, port and dirrectory of file system mount.
    ///
    /// Every process spawned on the node will have the same host and port.
    ///
    /// Here `storage_mount` specifies dirrectory within which process can manipulate with files.
    pub fn new(host: &str, port: u16, storage_mount: &str) -> Self {
        let max_buffer_size = 4 << 10;
        let (messages_sender, network_receiver) = mpsc::channel(max_buffer_size);

        let (network_sender, messages_receiver) = mpsc::channel(max_buffer_size);

        let network_handler =
            network::handle(messages_sender, messages_receiver, host.to_owned(), port);

        let (to_system_sender, from_process_receiver) = mpsc::channel(max_buffer_size);

        let mut system = Self {
            scheduled: Vec::new(),
            process_senders: HashMap::new(),
            from_process_receiver,
            to_system_sender,
            network_sender,
            network_receiver,
            max_buffer_size,
            host: host.to_owned(),
            port,
            mount_dir: storage_mount.to_owned(),
        };

        system.spawn(Box::pin(network_handler));

        system
    }

    /// Allows to spawn asynchronous activity on the node.
    ///
    /// Spawned activity will be executed together with added processes after call to
    /// [run][Node::run] method.
    pub fn spawn(&mut self, future: impl Future<Output = ()> + Send + 'static) {
        self.scheduled.push(Box::pin(future));
    }

    /// Allows to add process with specified name.
    ///
    /// Refer to [`Process`][crate::Process] documentation
    /// for mode details.
    pub fn add_process<P: Process + 'static>(
        &mut self,
        process: P,
        name: String,
    ) -> IOProcessWrapper<P> {
        let process_ref = Arc::new(RwLock::new(process));

        let (local_proc_sender, local_user_receiver) = mpsc::channel(self.max_buffer_size);
        let (local_user_sender, local_proc_receiver) = mpsc::channel(self.max_buffer_size);

        let address = Address {
            host: self.host.clone(),
            port: self.port,
            process_name: name.clone(),
        };

        let process_wrapper = ProcessWrapper {
            process_ref: process_ref.clone(),
            address: address.clone(),
        };

        let (to_proc_sender, from_proc_receiver) = mpsc::channel(self.max_buffer_size);

        let io_process_wrapper = IOProcessWrapper {
            wrapper: process_wrapper,
            sender: local_user_sender,
            receiver: local_user_receiver,
            system_sender: Some(self.to_system_sender.clone()),
            proc_name: name.clone(),
        };

        let process_manager_config = ProcessManagerConfig {
            address,
            process: process_ref,
            local_sender: local_proc_sender,
            local_receiver: local_proc_receiver,
            system_sender: self.to_system_sender.clone(),
            system_receiver: from_proc_receiver,
            network_sender: self.network_sender.clone(),
            max_buffer_size: self.max_buffer_size,
            mount_dir: self.mount_dir.clone(),
        };

        let proc_manager = ProcessManager::new(process_manager_config);

        if self.process_senders.contains_key(&name) {
            panic!("Trying to add existing process with name '{}'", name);
        }

        self.process_senders.insert(name, to_proc_sender);

        self.spawn(Box::pin(proc_manager.run()));

        io_process_wrapper
    }

    /// Run [spawned][crate::RealNode::spawn] asynchronous activities and [processes][crate::Process].
    ///
    /// Method will be blocked until all processes are [stopped][crate::Context::stop].
    pub fn run(mut self) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_time()
            .enable_io()
            .build()
            .expect("Can not create the runtime");

        // Run event loop and all spawned activities.
        runtime.block_on(async move {
            let working_processes =
                AtomicU32::new(self.process_senders.len().try_into().unwrap());
            // Spawn scheduled activities.
            for shed in self.scheduled {
                tokio::spawn(shed);
            }

            loop {
                tokio::select! {
                    Some(msg) = self.network_receiver.recv() => {
                        let sender = self.process_senders.get(&msg.to.process_name);

                        if let Some(sender) = sender {
                            let _ = sender.send(FromSystemMessage::NetworkMessage(msg)).await;
                        }
                    },
                    Some(msg) = self.from_process_receiver.recv() => {
                        match msg {
                            ToSystemMessage::ProcessStopped(proc_name) => {
                                let sender = self.process_senders.remove(&proc_name);

                                if let Some(sender) = sender {
                                    let _ = sender
                                     .send(FromSystemMessage::Suspend())
                                     .await;

                                    working_processes.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

                                    // Then all processes are stopped and we are done.
                                    if working_processes.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    else => break // All channels are closed.
                }
            }
        });
    }
}
