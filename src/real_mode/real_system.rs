//! Definition of [`RealSystem`] and [`RealSystemConfig`].

use std::sync::{Arc, RwLock};

use tokio::sync::mpsc::{self, Sender};

use crate::common::{
    actions::{ProcessAction, TimerBehavior},
    process::{Process, ProcessWrapper},
};

use super::{
    events::Event,
    network::{
        grpc_messenger::GRpcMessenger, manual_resolver::ManualResolver,
        network_manager::NetworkManager, resolver::AddressResolver,
    },
    process_manager::ProcessManager,
    time::{basic_timer_setter::BasicTimerSetter, time_manager::TimeManager},
};

pub use super::network::defs::Address;

/// Represents policy for resolving [`network address`][`crate::Address`] by [`process`][`crate::Process`] name.
///
/// It allows to know [`network address`][`Address`] of [user-process][`Process`]
/// to send [messages][`crate::Message`] to it.
///
/// Note that address resolving does not filter out not registered processes in general[^note].
/// It means user-process can receive messages from malicious processes too,
/// which can use or not use [`framework`](https://github.com/egnees/dsbuild);
///
/// [^note]: add some kind of protection against malicious processes in future.
pub enum AddressResolvePolicy {
    /// - Only processes with names corresponds[addresses][`Address`] in `resolve_list` will be resolved.
    /// - Attempts to send [message][`crate::common::message::Message`] to the processes which name not in `resolve_list` will lead to error.
    /// - All process names in the `resolve_list` must be unique.
    /// - [Messages][`crate::common::message::Message`] from not registered processes will be accepted and delivered too.
    ///
    /// Remark: probably in future there will be possibility
    /// to add [resolve records][`Address`] directly from [user-process][`Process`]
    /// using [context][`crate::common::context::Context`].
    Manual {
        /// List of [process names][`Address::process_name`] and their [addresses][`Address`],
        /// which will be used to send [messages][`crate::common::message::Message] to them.
        resolve_list: Vec<Address>,
    },
}

/// Represents configuration of [`RealSystem`].
pub struct RealSystemConfig {
    /// Max number of threads which will be used to handle events inside of the [`RealSystem`].
    max_threads: usize,

    /// Max size of buffer of pending events inside of the [`RealSystem`].
    event_buffer_size: usize,

    /// Policy for resolving [network addresses][`Address`] by process name.
    resolve_policy: AddressResolvePolicy,

    /// Host which will be used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
    host: String,

    /// Port which will be used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
    port: u16,
}

impl RealSystemConfig {
    /// Default size of the event buffer inside of the [`RealSystem`].
    pub const DEFAULT_EVENT_BUFFER_SIZE: usize = 1024;

    /// Default number of threads, which are used to handle events inside of the [`RealSystem`].
    pub const DEFAULT_MAX_THREADS: usize = 1;

    /// Creates new instance of [`RealSystemConfig`].
    ///
    /// * `max_threads` - Specifies max number of threads which will be used by [`RealSystem`] to handle events.
    ///     This value must be greater than zero.
    /// * `event_buffer_size` - Specifies size of the pending events buffer inside of the [`RealSystem`].
    ///     If the buffer is full, all threads will be blocked at the moment of sending event to the buffer,
    ///     while some old event won`t be processed.
    ///
    ///     This value must be greater than zero.
    /// * `resolve_policy` - Specifies policy for resolving [network addresses][`Address`] by process name.
    /// * `host` - Specifies host which will be used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
    /// * `port` - Specifies port which will be used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
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
            Ok(RealSystemConfig {
                max_threads,
                event_buffer_size,
                resolve_policy,
                host,
                port,
            })
        }
    }

    /// Alias for [`RealSystemConfig::new`] method, which creates new [`RealSystemConfig`]
    /// with specified number of threads, used to handle events inside of [`RealSystem`].
    ///
    /// Instead of `event_buffer_size` used [`RealSystemConfig::DEFAULT_EVENT_BUFFER_SIZE`].
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

    /// Alias for [`RealSystemConfig::new`] method, which creates new [`RealSystemConfig`]
    /// with specified size of buffer, which is used to store pending events inside of [`RealSystem`].
    ///
    /// Instead of `max_threads` used [`RealSystemConfig::DEFAULT_MAX_THREADS`].
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

    /// Alias for [`RealSystemConfig::new`] method, which creates new [`RealSystemConfig`]
    /// with default parameters.
    ///
    /// Instead of `max_threads` used [`RealSystemConfig::DEFAULT_MAX_THREADS`],
    /// and instead of `event_buffer_size` used [`RealSystemConfig::DEFAULT_EVENT_BUFFER_SIZE`].
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

/// Represents real system, which is responsible
/// for interacting with [`user-processes`][`Process`], time, network, and other [OS](https://en.wikipedia.org/wiki/Operating_system) features.
pub struct RealSystem {
    /// Represents [network address][`AddressResolver`] resolver,
    /// which is configured according to provided by [`RealSystemConfig`]
    /// [resolve policy][`RealSystemConfig::resolve_policy`].
    resolver: Box<dyn AddressResolver>,

    /// Represents [process manager][`ProcessManager`], which is used to manage [user-processes][`Process`].
    process_manager: ProcessManager,

    /// Represents [time_manager][`TimeManager`], which is used to work with time and set timers.
    time_manager: TimeManager<BasicTimerSetter>,

    /// Represents [network_manager][`NetworkManager`], which is used to work with network.
    network_manager: NetworkManager<GRpcMessenger>,

    /// Corresponds to [`RealSystemConfig::max_threads`].
    max_threads: usize,

    /// Corresponds to [`RealSystemConfig::event_buffer_size`].
    event_buffer_size: usize,

    /// Corresponds to [`RealSystemConfig::host`],
    /// which is used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
    host: String,

    /// Corresponds to [`RealSystemConfig::port`],
    /// which is used by [`RealSystem`] to listen for the incoming [messages][`crate::common::message::Message`].
    port: u16,
}

impl RealSystem {
    /// Creates new instance of [`RealSystem`] from [`RealSystemConfig`].
    pub fn new(config: RealSystemConfig) -> Result<Self, String> {
        // Create process manager.
        let process_manager = ProcessManager::default();

        // Create time manager.
        let time_manager = TimeManager::new();

        // Create network manager.
        let network_manager = NetworkManager::default();

        // Create resolver.
        let resolver = match config.resolve_policy {
            AddressResolvePolicy::Manual {
                resolve_list: trusted,
            } => Box::new(
                ManualResolver::from_trusted_list(trusted).expect("Can not create resolver"),
            ),
        };

        // Build and return created system.
        Ok(RealSystem {
            resolver,
            process_manager,
            time_manager,
            network_manager,
            max_threads: config.max_threads,
            event_buffer_size: config.event_buffer_size,
            host: config.host,
            port: config.port,
        })
    }

    /// Add new [user-process][`Process`] to the system.
    /// Names of processes must be unique.
    ///
    /// # Returns
    ///
    /// - [`Ok(ProcessWrapper)`][`ProcessWrapper`] contains wrapper which wraps passed `process`
    ///     and allows to [get read access][`ProcessWrapper::read`] to the `process`.
    /// - [`Err(String)`][`Err`] if process with the same name already exists in the system.
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

    /// Assistant function which is used to handle [process actions][`ProcessAction`] list.
    /// Applies [`RealSystem::handle_process_action`] to every action inside.
    fn handle_process_actions(
        &mut self,
        actions: &[ProcessAction],
        sender: &Sender<Event>,
    ) -> Result<(), String> {
        actions
            .iter()
            .try_for_each(|action| self.handle_process_action(action, sender))
    }

    /// Handle one [process action][`ProcessAction`].
    ///
    /// It performs corresponding call to the [`NetworkManager`] or [`TimeManager`] or other OS interact actor[^note],
    /// passing clone of `sender` to them, to receive [`Event`] in response to [action][`ProcessAction`],
    /// that will be handled by [user-process][`Process`] using [`ProcessManager`],
    /// which will generate new [action][`ProcessAction`] and so on.
    ///
    /// [^note]: There are no other interact actors for now.
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
                self.network_manager
                    .send_message(sender, receiver, msg.clone());
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
                self.time_manager.set_timer(
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
                self.time_manager.cancel_timer(process_name, timer_name);
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

    /// Returns [future][`core::future::Future`] which execution will lead to loop,
    /// in which [`RealSystem`] will wait for incoming [events][`Event`] and handle them.
    ///
    /// Every [event][`Event`] will be handled by [process_manager][`RealSystem::process_manager`]
    /// and produce few [actions][`ProcessAction`] which will be handled by [`RealSystem`]
    /// and will lead to interaction with OS and appearing of new [events][`Event`],
    /// which also need to be handled, and so on.
    ///
    /// The loop will be stopped when where are no communication channels[^note] with OS,
    /// which can produce new events.
    ///
    /// This can be achieved only in case when all [user-processes][`Process`] are stopped by user.
    ///
    /// # Returns
    ///
    /// [future][`core::future::Future`], which execution leads to[^note1]:
    ///
    /// - Will return [`Err`] only is case of runtime panics, which must be possible only if there are some
    ///     framework implementation errors. This will lead to the whole runtime will panic.
    /// - In case of receiving [`Err`] from [user-processes][`Process`], or from OS interaction actor,
    ///     error must be logged on the screen, but runtime still will continue to process events.
    ///     If user wants to stop the runtime, [user-process][`Process`] need panic.
    ///
    /// [^note]: Essentially communication channels with OS are organized using only one [multi-channel][`tokio::sync::mpsc::channel`],
    /// which will have one [receiver][`tokio::sync::mpsc::Receiver`] end and few [senders][`Sender`] ends. Receiver end will be holden by [`RealSystem`] and sender ends will be holden by OS interaction actors,
    /// like [`NetworkManager`] and [`TimeManager`]. For example, every timer, produced by [`TimeManager`], will have one [sender end][`Sender`],
    /// and [network listener][`NetworkManager`] also will hold one [sender end][`Sender`]. After timer fired, or listener stops,
    /// [sender][`Sender`] will be dropped. After all [senders][`Sender`] are dropped, loop will be ended.
    ///
    ///
    /// [^note1]: This behavior must be checked.
    async fn work(&mut self) -> Result<(), String> {
        // Create send and receive ends of channel.
        let (event_sender, mut event_receiver) = mpsc::channel(self.event_buffer_size);

        // Send system started event.
        event_sender
            .send(Event::SystemStarted {})
            .await
            .map_err(|e| e.to_string())?;

        // Start listen for incoming connections.
        self.network_manager
            .start_listen(self.host.clone(), self.port, event_sender.clone())?;

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
                self.time_manager.cancel_all_timers();
                self.network_manager.stop_listen();

                // Drop common sender.
                sender_option = None;
            } // After that there should be no new events in the channel.
        }

        Ok(())
    }

    /// Runs the [system][`RealSystem`] using [asynchronous runtime][tokio::runtime::Runtime].
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
