use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::common::actions::ProcessAction;
use crate::common::process::{Process, ProcessState};

use super::events::Event;
use super::real_context::RealContext;

#[derive(Default)]
pub struct ProcessManager {
    process_info: HashMap<String, (ProcessState, Arc<RwLock<dyn Process>>)>,
    active_process: u32,
}

impl ProcessManager {
    fn is_active(state: ProcessState) -> bool {
        state == ProcessState::Running
    }

    pub fn active_count(&self) -> u32 {
        self.active_process
    }

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

                let mut process = self.get_process(&process_name)?;

                let mut context = RealContext::new(process_name);

                process.on_timer(timer_name, &mut context)?;

                new_actions.append(&mut context.get_actions());
            }
            Event::MessageReceived { msg, from, to } => {
                let state = self.get_state(to.as_str())?;
                if !Self::is_active(*state) {
                    return Ok(vec![]);
                }

                let mut process = self.get_process(&to)?;

                let mut context = RealContext::new(to);

                process.on_message(msg, from, &mut context)?;

                new_actions.append(&mut context.get_actions());
            }
            Event::SystemStarted {} => {
                self.active_process = 0;

                for (process_name, (state, process)) in self.process_info.iter_mut() {
                    let mut context = RealContext::new(process_name.clone());

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
            let insert_result = self
                .process_info
                .insert(process_name.clone(), (ProcessState::Inited, process_impl));
            if insert_result.is_some() {
                panic!(
                    "Can not add process: process with name {} is present, but must not",
                    &process_name
                );
            }

            Ok(())
        }
    }

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
