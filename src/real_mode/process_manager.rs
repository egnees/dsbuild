//! Definition of [`ProcessManager`], which is responsible for managing [user processes][`Process`].

use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::common::actions::ProcessAction;
use crate::common::process::{Address, Process, ProcessState};

use super::events::Event;
use super::real_context::RealContext;

/// Process manager is responsible for managing [user processes][`Process`].
///
/// It manages states of user processes and maintains number of active processes.
/// [`ProcessManager`] is also responsible for handling system [events][`Event`] and receiving
/// response [actions][ProcessAction] of [user processes][`Process`].
pub struct ProcessManager {
    /// Host of the system.
    host: String,
    /// Port of the system.
    port: u16,
    /// Holds mapping from process name to (process state, process implementation pointer) pair.
    process_info: HashMap<String, (ProcessState, Arc<RwLock<dyn Process>>)>,
    /// Number of active processes.
    active_process: u32,
}

impl ProcessManager {
    /// Creates a new [`ProcessManager`] instance.
    pub fn new(host: String, port: u16) -> Self {
        ProcessManager {
            host,
            port,
            process_info: HashMap::new(),
            active_process: 0,
        }
    }

    /// Check if process state corresponds to active process.
    fn is_active(state: ProcessState) -> bool {
        state == ProcessState::Running
    }

    /// Returns number of active processes.
    ///
    /// This function is used by [`RealSystem`][`super::real_system::RealSystem`]
    /// to stop interaction with OS when there are no active processes.
    pub fn active_count(&self) -> u32 {
        self.active_process
    }

    /// Assistant function for getting reference on process implementation by process name.
    fn get_process(
        &mut self,
        process_name: &str,
    ) -> Result<RwLockWriteGuard<dyn Process + 'static>, String> {
        if self.process_info.contains_key(process_name) {
            let (_, proc_ref) = self.process_info.get(process_name).unwrap();
            let lock_wrapper = proc_ref.write();
            lock_wrapper.map_err(|e| e.to_string())
        } else {
            Err(format!(
                "Can not get process with name {}",
                process_name.to_owned()
            ))
        }
    }

    /// Assistant function for getting mutable reference on process state by process name.
    fn get_state(&mut self, process_name: &str) -> Result<&mut ProcessState, String> {
        if self.process_info.contains_key(process_name) {
            let (state, _) = self.process_info.get_mut(process_name).unwrap();

            Ok(state)
        } else {
            Err(format!(
                "Can not get process with name {}: process is not present",
                process_name.to_owned()
            ))
        }
    }

    /// Function is used for handling system [events][`Event`] and receiving response [actions][ProcessAction] by [user processes][`Process`].
    pub fn handle_event(&mut self, event: Event) -> Result<Vec<ProcessAction>, String> {
        let mut new_actions = Vec::default();

        match event {
            Event::TimerFired {
                process_name,
                timer_name,
            } => {
                let state = self.get_state(process_name.as_str())?;
                if !Self::is_active(*state) {
                    return Ok(vec![]);
                }

                let mut context = RealContext::new(Address::new(
                    self.host.clone(),
                    self.port,
                    process_name.clone(),
                ));

                let mut process = self.get_process(&process_name)?;

                process.on_timer(timer_name, &mut context)?;

                new_actions.append(&mut context.get_actions());
            }
            Event::MessageReceived { msg, from, to } => {
                let receiver_address = Address::new(self.host.clone(), self.port, to.clone());

                let state = self.get_state(&to)?;
                if !Self::is_active(*state) {
                    return Ok(vec![]);
                }

                let mut process = self.get_process(&to)?;

                let mut context = RealContext::new(receiver_address);

                process.on_message(msg, from, &mut context)?;

                new_actions.append(&mut context.get_actions());
            }
            Event::SystemStarted {} => {
                self.active_process = 0;

                for (process_name, (state, process)) in self.process_info.iter_mut() {
                    let process_address =
                        Address::new(self.host.clone(), self.port, process_name.to_owned());

                    let mut context = RealContext::new(process_address);

                    *state = ProcessState::Running;
                    self.active_process += 1;

                    process
                        .write()
                        .map_err(|e| e.to_string())?
                        .on_start(&mut context)?;

                    new_actions.append(&mut context.get_actions());
                }
            }
        };

        Ok(new_actions)
    }

    /// Register [process][`Process`] in the [`ProcessManager`] with specified `process_name` and `process_impl`.
    ///
    /// Here `process_impl` is a link to the implementation of the [user process][`Process`],
    /// provided by the user.
    ///
    /// # Returns
    ///
    /// - [`Ok`] in case of success.
    /// - [`Err`] in case of process with the same `process_name` already exists.
    pub fn add_process(
        &mut self,
        process_name: String,
        process_impl: Arc<RwLock<dyn Process>>,
    ) -> Result<(), String> {
        if self.process_info.contains_key(&process_name) {
            Err(format!(
                "Can not add process: process with name {} already exists",
                process_name.as_str()
            ))
        } else {
            let insert_result = self.process_info.insert(
                process_name.clone(),
                (ProcessState::Initialized, process_impl),
            );
            if insert_result.is_some() {
                panic!(
                    "Can not add process: process with name {} is present, but must not",
                    &process_name
                );
            }

            Ok(())
        }
    }

    /// Stop the process with the specified `process_name`.
    pub fn stop_process(&mut self, process_name: &str) -> Result<(), String> {
        let state = self.get_state(process_name)?;

        if *state == ProcessState::Stopped {
            return Ok(());
        }

        let old_state = *state;

        *state = ProcessState::Stopped;

        if Self::is_active(old_state) {
            self.active_process -= 1;
        }

        Ok(())
    }
}
