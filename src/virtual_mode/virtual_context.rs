use crate::common::{
    actions::{ProcessAction, StopPolicy, TimerBehavior},
    context::Context,
    message::Message,
    process::Address,
};

#[derive(Clone)]
pub struct VirtualContext {
    pub process_address: Address,
    pub actions: Vec<ProcessAction>,
}

impl Context for VirtualContext {
    fn set_timer(&mut self, name: String, delay: f64) {
        let action = ProcessAction::TimerSet {
            process_name: self.process_address.process_name.clone(),
            timer_name: name,
            delay,
            behavior: TimerBehavior::OverrideExisting,
        };

        self.actions.push(action);
    }
    fn set_timer_once(&mut self, name: String, delay: f64) {
        let action = ProcessAction::TimerSet {
            process_name: self.process_address.process_name.clone(),
            timer_name: name,
            delay,
            behavior: TimerBehavior::SetOnce,
        };

        self.actions.push(action);
    }
    fn cancel_timer(&mut self, name: String) {
        let action = ProcessAction::TimerCancelled {
            process_name: self.process_address.process_name.clone(),
            timer_name: name,
        };

        self.actions.push(action);
    }
    fn send_message(&mut self, msg: Message, to: Address) {
        let action = ProcessAction::MessageSent {
            msg,
            from: self.process_address.clone(),
            to,
        };

        self.actions.push(action);
    }
    fn stop_process(&mut self) {
        let action = ProcessAction::ProcessStopped {
            process_name: self.process_address.process_name.clone(),
            policy: StopPolicy::Immediately,
        };

        self.actions.push(action);
    }
}
