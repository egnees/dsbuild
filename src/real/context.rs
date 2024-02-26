//! Definition of [`RealContext`].

use crate::common::actions::ProcessAction;
use crate::common::process::Address;

#[derive(Clone)]
pub struct RealContextImpl {
    process_address: Address,
    actions: Vec<ProcessAction>,
}

#[derive(Clone)]
pub struct RealContext {}

impl RealContextImpl {
    pub fn new(process_address: Address) -> Self {
        RealContextImpl {
            process_address,
            actions: Vec::default(),
        }
    }

    pub fn get_actions(&self) -> Vec<ProcessAction> {
        self.actions.clone()
    }
}
