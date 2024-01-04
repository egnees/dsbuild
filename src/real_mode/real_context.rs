//! Definition of [`RealContext`].

use crate::common::actions::{ProcessAction, StopPolicy, TimerBehavior};
use crate::common::context::Context;
use crate::common::message::Message;

#[derive(Clone)]
pub struct RealContext {
    process_name: String,
    actions: Vec<ProcessAction>,
}

impl RealContext {
    pub fn new(process_name: String) -> Self {
        RealContext {
            process_name,
            actions: Vec::default(),
        }
    }

    pub fn get_actions(&self) -> Vec<ProcessAction> {
        self.actions.clone()
    }
}

impl Context for RealContext {
    fn set_timer(&mut self, name: String, delay: f64) {
        let action = ProcessAction::TimerSet {
            process_name: self.process_name.clone(),
            timer_name: name,
            delay,
            behavior: TimerBehavior::OverrideExisting,
        };

        self.actions.push(action);
    }

    fn set_timer_once(&mut self, name: String, delay: f64) {
        let action = ProcessAction::TimerSet {
            process_name: self.process_name.clone(),
            timer_name: name,
            delay,
            behavior: TimerBehavior::SetOnce,
        };

        self.actions.push(action);
    }

    fn cancel_timer(&mut self, name: String) {
        let action = ProcessAction::TimerCancelled {
            process_name: self.process_name.clone(),
            timer_name: name,
        };

        self.actions.push(action);
    }

    fn send_message(&mut self, msg: Message, to: String) {
        let action = ProcessAction::MessageSent {
            msg,
            from: self.process_name.clone(),
            to,
        };

        self.actions.push(action);
    }

    fn stop_process(&mut self) {
        let action = ProcessAction::ProcessStopped {
            process_name: self.process_name.clone(),
            policy: StopPolicy::Immediately,
        };

        self.actions.push(action);
    }
}
