use std::sync::{Arc, RwLock};

use tokio::sync::mpsc::{self, Sender};

use crate::common::{
    actions::{ProcessAction, TimerBehavior},
    process::{Process, ProcessWrapper},
};

use super::{
    events::Event,
    network::{
        defs::Address, manual_resolver::ManualResolver, network_manager, resolver::AddressResolver,
    },
    process_manager::ProcessManager,
    time::time_manager,
};

pub enum AddressResolvePolicy {
    Manual { trusted: Vec<Address> },
}

pub struct SystemConfig {
    max_threads: usize,
    event_buffer_size: usize,
    resolve_policy: AddressResolvePolicy,
    host: String,
    port: u16,
}

impl SystemConfig {
    const DEFAULT_EVENT_BUFFER_SIZE: usize = 1024;
    const DEFAULT_MAX_THREADS: usize = 1;

    pub fn new(
        max_threads: usize,
        event_buffer_size: usize,
        resolve_policy: AddressResolvePolicy,
        host: String,
        port: u16,
    ) -> Result<Self, String> {
        if event_buffer_size == 0 {
            Err("Event buffer size can not be 0".to_owned())
        } else if max_threads == 0 {
            Err("Max threads can not be 0".to_owned())
        } else {
            Ok(SystemConfig {
                max_threads,
                event_buffer_size,
                resolve_policy,
                host,
                port,
            })
        }
    }

    pub fn new_with_max_threads(
        max_threads: usize,
        resolve_policy: AddressResolvePolicy,
        host: String,
        port: u16,
    ) -> Result<Self, String> {
        Self::new(
            max_threads,
            Self::DEFAULT_EVENT_BUFFER_SIZE,
            resolve_policy,
            host,
            port,
        )
    }

    pub fn new_with_buffer_size(
        event_buffer_size: usize,
        resolve_policy: AddressResolvePolicy,
        host: String,
        port: u16,
    ) -> Result<Self, String> {
        Self::new(
            Self::DEFAULT_MAX_THREADS,
            event_buffer_size,
            resolve_policy,
            host,
            port,
        )
    }

    pub fn default(
        resolve_policy: AddressResolvePolicy,
        host: String,
        port: u16,
    ) -> Result<Self, String> {
        Self::new(
            Self::DEFAULT_MAX_THREADS,
            Self::DEFAULT_EVENT_BUFFER_SIZE,
            resolve_policy,
            host,
            port,
        )
    }
}

pub struct System {
    resolver: Box<dyn AddressResolver>,
    process_manager: ProcessManager,
    max_threads: usize,
    event_buffer_size: usize,
    host: String,
    port: u16,
}

impl System {
    pub fn new(config: SystemConfig) -> Result<Self, String> {
        // Create process manager.
        let process_manager = ProcessManager::default();

        // Create resolver.
        let resolver = match config.resolve_policy {
            AddressResolvePolicy::Manual { trusted } => ManualResolver::from_trusted_list(trusted)?,
        };

        // Create system.
        Ok(System {
            resolver: Box::new(resolver),
            process_manager,
            max_threads: config.max_threads,
            event_buffer_size: config.event_buffer_size,
            host: config.host,
            port: config.port,
        })
    }

    pub fn add_process<P: Process + 'static>(
        &mut self,
        process_name: &str,
        process: P,
    ) -> Result<ProcessWrapper<P>, String> {
        // Define process address.
        let process_address = Address {
            host: self.host.clone(),
            port: self.port,
            process_name: process_name.to_owned(),
        };

        // Add process to the resolver.
        self.resolver.add_record(&process_address)?;

        // Create process lock.
        let process_lock = Arc::new(RwLock::new(process));

        // Create process wrapper.
        let process_wrapper = ProcessWrapper {
            process_ref: process_lock.clone(),
        };

        // Add process to process manager.
        self.process_manager
            .add_process(process_name.to_owned(), process_lock)?;

        // Return process wrapper.
        Ok(process_wrapper)
    }

    fn handle_process_actions(
        &mut self,
        actions: &[ProcessAction],
        sender: &Sender<Event>,
    ) -> Result<(), String> {
        actions
            .iter()
            .try_for_each(|action| self.handle_process_action(action, sender))
    }

    fn handle_process_action(
        &mut self,
        action: &ProcessAction,
        sender: &Sender<Event>,
    ) -> Result<(), String> {
        match action {
            // Process message sent action.
            ProcessAction::MessageSent { msg, from, to } => {
                // Get sender and receiver addresses.
                let sender = self.resolver.resolve(from)?;
                let receiver = self.resolver.resolve(to)?;

                // Send message using network manager.
                network_manager::send_message(sender, receiver, msg.clone());
            }

            // Process timer set action.
            ProcessAction::TimerSet {
                process_name,
                timer_name,
                delay,
                behavior,
            } => {
                // Get overwrite policy.
                let overwrite = match behavior {
                    TimerBehavior::SetOnce => false,
                    TimerBehavior::OverrideExisting => true,
                };

                // Set timer.
                time_manager::set_timer(
                    sender.clone(),
                    process_name,
                    timer_name,
                    *delay,
                    overwrite,
                );
            }

            // Process timer cancelled action.
            ProcessAction::TimerCancelled {
                process_name,
                timer_name,
            } => {
                // Cancel timer.
                time_manager::cancel_timer(process_name, timer_name);
            }

            // Process request to stop the process.
            ProcessAction::ProcessStopped {
                process_name,
                policy: _,
            } => {
                self.process_manager.stop_process(process_name)?;
            }
        }

        Ok(())
    }

    async fn work(&mut self) -> Result<(), String> {
        // Initialize time manager.
        time_manager::init();

        // Intialize network manager.
        network_manager::init();

        // Create send and receive ends of channel.
        let (event_sender, mut event_receiver) = mpsc::channel(self.event_buffer_size);

        // Send system started event.
        event_sender
            .send(Event::SystemStarted {})
            .await
            .map_err(|e| e.to_string())?;

        // Start listen for incomming connections.
        network_manager::start_listen(self.host.clone(), self.port, event_sender.clone())?;

        // Move event_sender to sender option
        let mut sender_option = Some(event_sender);

        // Start event dispatching loop.
        while let Some(event) = event_receiver.recv().await {
            // Then there is a sender certainly.
            let sender = sender_option.as_ref().expect("Incorrect implementation");

            // Get process actions.
            let actions = self.process_manager.handle_event(event)?;
            self.handle_process_actions(&actions, sender)?;

            // If there is no active processes, we can shutdown the system.
            if self.process_manager.active_count() == 0 {
                time_manager::cancel_all_timers();
                network_manager::stop_listen();

                // Drop common sender.
                sender_option = None;
            } // After that there should be no new events in the channel.
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), String> {
        // Create runtime according to specified number of threads.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.max_threads)
            .enable_io()
            .build()
            .expect("Can not create the runtime");

        // Start runtime.
        runtime.block_on(self.work())
    }
}
