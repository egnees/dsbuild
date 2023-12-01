use crate::common::{process::{Process, ProcessState}, actions::{ProcessAction, TimerBehavior, StopPolicy}};

use dslab_mp::{process::Process as SimulationProcess, message::Message as SimulationMessage, context::Context as SimulationContext};

use super::virtual_context::VirtualContext;

#[derive(Clone)]
pub struct ProcessWrapper<P: Process> {
    user_process: Box<P>,
    process_state: ProcessState,
}

impl<P: Process> ProcessWrapper<P> {
    pub fn new(user_process: Box<P>) -> Self {
        Self {
            user_process,
            process_state: ProcessState::Inited,
        }
    }

    // Implement and use it in future to get reference on the process  
    // pub fn get_process_ref(&self) -> &P {
    //     self.user_process.as_ref()
    // }

    fn virtual_context() -> VirtualContext {
        VirtualContext { actions: Vec::new() }
    }

    fn handle_process_actions(&mut self, actions: Vec<ProcessAction>, ctx: &mut SimulationContext) {
        for action in actions {
            if self.process_state == ProcessState::Stopped {
                break;
            }
            match action {
                ProcessAction::MessageSent { msg, to } => {
                    ctx.send(msg, to);
                },
                ProcessAction::ProcessStopped { policy } => {
                    match policy {
                        StopPolicy::Defer => {
                            assert!(self.process_state == ProcessState::Running || self.process_state == ProcessState::Stopping);
                            self.process_state = ProcessState::Stopping;
                        },
                        StopPolicy::Immediately => {
                            assert!(self.process_state != ProcessState::Inited);
                            self.process_state = ProcessState::Stopped;
                        }
                    }
                },
                ProcessAction::TimerSet { name, delay, behavior } => {
                    match behavior {
                        TimerBehavior::SetOnce => ctx.set_timer_once(name.as_str(), delay),
                        TimerBehavior::OverrideExisting => ctx.set_timer(name.as_str(), delay),
                    }
                },
                ProcessAction::TimerCancelled { name } => ctx.cancel_timer(name.as_str()),
            }
        }
    }
}

impl<P: Process> SimulationProcess for ProcessWrapper<P> {
    fn on_message(&mut self, msg: SimulationMessage, from: String, ctx: &mut SimulationContext) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped || self.process_state == ProcessState::Stopping {
            return Ok(());
        }

        let mut virt_ctx = Self::virtual_context();

        let result = self.user_process.on_message(msg.into(), from, &mut virt_ctx);
        
        self.handle_process_actions(virt_ctx.actions, ctx);
        
        result
    }

    fn on_local_message(&mut self, msg: SimulationMessage, ctx: &mut SimulationContext) -> Result<(), String> {
        assert_eq!(msg.tip, "START");

        assert_eq!(self.process_state, ProcessState::Inited);
        
        self.process_state = ProcessState::Running;

        let mut virt_ctx = Self::virtual_context();
        
        let result = self.user_process.on_start(&mut virt_ctx);
        
        self.handle_process_actions(virt_ctx.actions, ctx);
        
        result
    }

    fn on_timer(&mut self, timer: String, ctx: &mut SimulationContext) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        let mut virt_ctx = Self::virtual_context();
        
        let result = self.user_process.on_timer(timer, &mut virt_ctx);
        
        self.handle_process_actions(virt_ctx.actions, ctx);
        
        result
    }
}

