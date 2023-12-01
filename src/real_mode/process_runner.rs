use crate::common::actions::{ProcessAction, TimerBehavior, StopPolicy};
use crate::common::process::Process;

use super::network_manager::NetworkManager;
use super::real_context::RealContext;
use super::timer_manager::TimerManager;
use super::events::Event;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone, Debug, PartialEq)]
enum ProcessState {
    Inited,
    Running,
    Stopping,
    Stopped
}

pub struct ProcessRunner {
    event_queue: Arc<Mutex<VecDeque<Event>>>,
    timer_manager: TimerManager,
    network_manager: NetworkManager,
    state: ProcessState,
}

pub struct RunConfig {
    pub host: String,
}

impl ProcessRunner {
    pub fn new(config: RunConfig) -> Result<Self, String> {
        let queue = Arc::new(Mutex::new(VecDeque::new())); 
        
        let runner = Self {
            event_queue: queue.clone(),
            timer_manager: TimerManager::new(queue.clone()),
            network_manager: NetworkManager::new(queue.clone(), 512, config.host, 0.1)?,
            state: ProcessState::Inited,
        };

        Ok(runner)
    }

    pub fn run<'a, P: Process>(&mut self, proc: &'a mut P) -> Result<(), String> {
        assert!(self.state == ProcessState::Inited, "Trying to run ProcessRunner twice");

        self.state = ProcessState::Running;

        self.event_queue.lock().unwrap().push_back(Event::SystemStarted {  });

        while !self.can_stop_process() {
            while !self.can_stop_process() {
                let event_opt = self.event_queue.lock().unwrap().pop_front();
                if let Some(event) = event_opt {
                    let actions_result = self.handle_event(proc, event);
                    if let Ok(actions) = actions_result {
                        self.handle_process_actions(actions);
                    } else {
                        self.stop(StopPolicy::Immediately);
                        return Err(actions_result.err().unwrap());
                    }
                } else {
                    break;
                }
            }
        }

        self.stop(StopPolicy::Immediately);

        Ok(())
    }

    fn stop(&mut self, policy: StopPolicy) {
        assert!(self.state == ProcessState::Running || self.state == ProcessState::Stopping);
        match policy {
            StopPolicy::Immediately => {
                self.timer_manager.cancel_all_timers();
                self.network_manager.stop_listen().expect("Unexpected panic in the network manager listening thread");
                self.event_queue.lock().unwrap().clear();
                self.state = ProcessState::Stopped;
            },
            StopPolicy::Defer => {
                self.network_manager.stop_listen().expect("Unexpected panic in the network manager listening thread");
                self.state = ProcessState::Stopping;
            }
        }
    }

    fn stopping_process(&self) -> bool {
        return self.state == ProcessState::Stopping;
    }

    fn stopped_process(&self) -> bool {
        return self.state == ProcessState::Stopped;
    }

    fn can_stop_process(&self) -> bool {
        if self.stopped_process() 
            || (self.stopping_process() 
                    && !self.timer_manager.have_timers() 
                    && self.event_queue.lock().unwrap().is_empty()) {
            true
        } else {
            false
        }
    }

    fn handle_process_action(&mut self, action: ProcessAction) {
        match action {
            ProcessAction::TimerSet { name, delay, behavior } => {
                let overwrite = match behavior {
                    TimerBehavior::SetOnce => false,
                    TimerBehavior::OverrideExisting => true,
                };
                self.timer_manager.set_timer(name.as_str(), delay, overwrite);
            },
            ProcessAction::TimerCancelled { name } => {
                self.timer_manager.cancel_timer(name.as_str());
            }
            ProcessAction::MessageSent { msg, to } => {
                self.network_manager.send_message(to, msg);
            },
            ProcessAction::ProcessStopped { policy } => {
                self.stop(policy);
            }
        }
    }

    fn handle_process_actions(&mut self, actions: Vec<ProcessAction>) {
        for action in actions {
            self.handle_process_action(action);
        }
    }

    fn handle_event<P: Process>(&mut self, proc: &mut P, event: Event) -> Result<Vec<ProcessAction>, String> {
        if self.stopped_process() {
            return Ok(Vec::new());
        }
        
        let mut ctx = RealContext {
            actions: Vec::new()
        };

        match event {
            Event::MessageReceived { msg, from } => {
                proc.on_message(msg, from, &mut ctx)?;
            },
            Event::SystemStarted {  } => {
                self.network_manager.start_listen()?;
                proc.on_start(&mut ctx)?;
            },
            Event::TimerFired { name } => {
                proc.on_timer(name, &mut ctx)?;
            }
        }

        Ok(ctx.actions)
    }
}

