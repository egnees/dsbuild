use crate::common::context::Context;
use crate::common::actions::{ProcessAction, TimerBehavior, StopPolicy};

pub struct RealContext {
    pub actions: Vec<ProcessAction>
}

impl Context for RealContext {
    fn set_timer(&mut self, name: String, delay: f64) {
        let action = 
                ProcessAction::TimerSet { name, delay, behavior: TimerBehavior::OverrideExisting };
        self.actions.push(action);
    }
    fn set_timer_once(&mut self, name: String, delay: f64) {
        let action = 
                ProcessAction::TimerSet { name, delay, behavior: TimerBehavior::SetOnce };
        self.actions.push(action);
    }
    fn cancel_timer(&mut self, name: String) {
        let action = 
                ProcessAction::TimerCancelled { name };
        self.actions.push(action);
    }
    fn stop_process(&mut self, immediately: bool) {
        let policy = if immediately {
            StopPolicy::Immediately
        } else {
            StopPolicy::Defer
        };
        let action = ProcessAction::ProcessStopped { policy };
        self.actions.push(action);
    }
}
