use std::cell::{Ref, RefMut};
use sugars::boxed;

use dslab_mp::{
    message::Message as SimulationMessage, network::Network, node::Node,
    system::System as Simulation,
};

use super::process_wrapper::ProcessWrapper;
use crate::common::process::Process;

pub struct VirtualSystem {
    simulation: Simulation,
}

impl VirtualSystem {
    pub fn new(seed: u64) -> Self {
        Self {
            simulation: Simulation::new(seed),
        }
    }

    // Network ---------------------------------------------------

    /// Returns a mutable reference to network.
    pub fn network(&self) -> RefMut<Network> {
        self.simulation.network()
    }

    // Node ------------------------------------------------------

    /// Adds a node to the simulation.
    ///
    /// Note that node names must be unique.
    pub fn add_node(&mut self, name: &str) {
        self.simulation.add_node(name);
    }

    /// Returns a list of node names in the simulation.
    pub fn nodes(&self) -> Vec<String> {
        self.simulation.nodes()
    }

    /// Crashes the specified node.
    ///
    /// All pending events created by the node will be discarded.
    /// The undelivered messages sent by the node will be dropped.
    /// All pending and future events destined to the node will be discarded.
    ///
    /// Processes running on the node are not cleared to allow working
    /// with processes after the crash (i.e. examine event log).
    pub fn crash_node(&mut self, node_name: &str) {
        self.simulation.crash_node(node_name);
    }

    /// Recovers the previously crashed node.
    ///
    /// Processes running on the node before the crash are cleared.
    /// The delivery of events to the node is enabled.
    pub fn recover_node(&mut self, node_name: &str) {
        self.simulation.recover_node(node_name);
    }

    /// Returns an immutable reference to the node.
    pub fn get_node(&self, name: &str) -> Option<Ref<Node>> {
        self.simulation.get_node(name)
    }

    /// Returns a mutable reference to the node.
    pub fn get_mut_node(&self, name: &str) -> Option<RefMut<Node>> {
        self.simulation.get_mut_node(name)
    }

    /// Checks if the node is crashed.
    pub fn node_is_crashed(&self, node: &str) -> bool {
        self.simulation.node_is_crashed(node)
    }

    // Process ------------------------------------------------------

    pub fn add_process<P: Process>(&mut self, name: &str, proc: &'static mut P, node: &str) {
        let proc_box = boxed!(proc.clone());
        let proc_wrapper = boxed!(ProcessWrapper::new(proc_box));
        self.simulation.add_process(name, proc_wrapper, node);
    }

    /// Start process
    /// Call on_start method of process
    pub fn start(&mut self, proc: &str, node: &str) {
        if let Some(mut node_ref) = self.get_mut_node(node) {
            node_ref.send_local_message(proc.to_string(), SimulationMessage::new("START", ""));
        }
    }

    /// Returns the names of all processes in the system.
    pub fn process_names(&self) -> Vec<String> {
        self.simulation.process_names()
    }

    /// Returns the number of messages sent by the process.
    pub fn sent_message_count(&self, proc: &str) -> u64 {
        self.simulation.sent_message_count(proc)
    }

    /// Returns the number of messages received by the process.
    pub fn received_message_count(&self, proc: &str) -> u64 {
        self.simulation.received_message_count(proc)
    }

    // Simulation -----------------------------------------------------

    /// Steps through the simulation until there are no pending events left.
    pub fn step_until_no_events(&mut self) {
        self.simulation.step_until_no_events()
    }
}
