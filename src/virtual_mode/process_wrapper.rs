use std::sync::{Arc, RwLock};

use crate::common::{
    actions::{ProcessAction, StopPolicy, TimerBehavior},
    process::{Process, ProcessState},
};

use dslab_mp::{
    context::Context as SimulationContext, message::Message as SimulationMessage,
    process::Process as SimulationProcess,
};

use super::virtual_context::VirtualContext;

#[derive(Clone)]
pub struct VirtualProcessWrapper<P: Process + Clone + 'static> {
    process_name: String,
    user_process: Arc<RwLock<P>>,
    process_state: ProcessState,
}

impl<P: Process + Clone + 'static> VirtualProcessWrapper<P> {
    pub fn new(process_name: String, process_impl: Arc<RwLock<P>>) -> Self {
        Self {
            process_name,
            user_process: process_impl,
            process_state: ProcessState::Inited,
        }
    }

    fn create_context(&self) -> VirtualContext {
        VirtualContext {
            process_name: self.process_name.clone(),
            actions: Vec::new(),
        }
    }

    fn handle_process_actions(&mut self, actions: Vec<ProcessAction>, ctx: &mut SimulationContext) {
        for action in actions {
            if self.process_state == ProcessState::Stopped {
                break;
            }
            match action {
                ProcessAction::MessageSent { msg, from: _, to } => {
                    ctx.send(msg.into(), to);
                }

                ProcessAction::ProcessStopped {
                    process_name: _,
                    policy,
                } => match policy {
                    StopPolicy::Immediately => {
                        assert!(self.process_state != ProcessState::Inited);
                        self.process_state = ProcessState::Stopped;
                    }
                },

                ProcessAction::TimerSet {
                    process_name: _,
                    timer_name,
                    delay,
                    behavior,
                } => match behavior {
                    TimerBehavior::SetOnce => ctx.set_timer_once(timer_name.as_str(), delay),
                    TimerBehavior::OverrideExisting => ctx.set_timer(timer_name.as_str(), delay),
                },

                ProcessAction::TimerCancelled {
                    process_name: _,
                    timer_name,
                } => ctx.cancel_timer(timer_name.as_str()),
            }
        }
    }
}

impl<P: Process + Clone + 'static> SimulationProcess for VirtualProcessWrapper<P> {
    fn on_message(
        &mut self,
        msg: SimulationMessage,
        from: String,
        ctx: &mut SimulationContext,
    ) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        let mut virt_ctx = self.create_context();

        let result = self
            .user_process
            .write()
            .expect("Can not read process, probably datarace detected")
            .on_message(msg.into(), from, &mut virt_ctx);

        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }

    fn on_local_message(
        &mut self,
        msg: SimulationMessage,
        ctx: &mut SimulationContext,
    ) -> Result<(), String> {
        assert_eq!(msg.tip, "START");

        assert_eq!(self.process_state, ProcessState::Inited);

        self.process_state = ProcessState::Running;

        let mut virt_ctx = self.create_context();

        let result = self
            .user_process
            .write()
            .expect("Can not read process, probably datarace detected")
            .on_start(&mut virt_ctx);

        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }

    fn on_timer(&mut self, timer: String, ctx: &mut SimulationContext) -> Result<(), String> {
        if self.process_state == ProcessState::Stopped {
            return Ok(());
        }

        let mut virt_ctx = self.create_context();

        let result = self
            .user_process
            .write()
            .expect("Can not read process, probably datarace detected")
            .on_timer(timer, &mut virt_ctx);

        self.handle_process_actions(virt_ctx.actions, ctx);

        result
    }
}
