//! Definition of [`VirtualSystem`].

use std::{
    cell::{Ref, RefMut},
    sync::{Arc, RwLock},
};

use dslab_mp::{
    message::Message as SimulationMessage, network::Network, node::Node,
    system::System as Simulation,
};

use super::process_wrapper::VirtualProcessWrapper;
use crate::common::process::{Process, ProcessWrapper};

/// Represents virtual system, which is responsible
/// for interacting with [`user-processes`][`Process`],
/// simulating time, network, and other [OS](https://en.wikipedia.org/wiki/Operating_system) features.
///
/// [`VirtualSystem`] uses [DSLab MP](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)
/// framework for simulation of network, time, etc.
pub struct VirtualSystem {
    inner: Simulation,
}

impl VirtualSystem {
    /// Creates new [`VirtualSystem`] with provided `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            inner: Simulation::new(seed),
        }
    }

    // Network ---------------------------------------------------

    /// Returns a mutable reference to network.
    pub fn network(&self) -> RefMut<Network> {
        self.inner.network()
    }

    // Node ------------------------------------------------------

    /// Adds a node to the simulation.
    ///
    /// Note that node names must be unique.
    pub fn add_node(&mut self, name: &str) {
        self.inner.add_node(name);
    }

    /// Returns a list of node names in the simulation.
    pub fn nodes(&self) -> Vec<String> {
        self.inner.nodes()
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
        self.inner.crash_node(node_name);
    }

    /// Recovers the previously crashed node.
    ///
    /// Processes running on the node before the crash are cleared.
    /// The delivery of events to the node is enabled.
    pub fn recover_node(&mut self, node_name: &str) {
        self.inner.recover_node(node_name);
    }

    /// Returns an immutable reference to the node.
    pub fn get_node(&self, name: &str) -> Option<Ref<Node>> {
        self.inner.get_node(name)
    }

    /// Returns a mutable reference to the node.
    pub fn get_mut_node(&self, name: &str) -> Option<RefMut<Node>> {
        self.inner.get_mut_node(name)
    }

    /// Checks if the node is crashed.
    pub fn node_is_crashed(&self, node: &str) -> bool {
        self.inner.node_is_crashed(node)
    }

    // Process ------------------------------------------------------

    /// Adds a process to the [`VirtualSystem`].
    ///
    /// # Panics
    ///
    /// - If `process name` is already used.
    pub fn add_process<P: Process + Clone + 'static>(
        &mut self,
        process_name: &str,
        process: P,
        node_name: &str,
    ) -> ProcessWrapper<P> {
        let process_ref = Arc::new(RwLock::new(process));

        let process_wrapper = ProcessWrapper {
            process_ref: process_ref.clone(),
        };

        let virtual_proc_wrapper = VirtualProcessWrapper::new(process_name.to_owned(), process_ref);

        let boxed_wrapper = Box::new(virtual_proc_wrapper);

        self.inner
            .add_process(process_name, boxed_wrapper, node_name);

        process_wrapper
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
        self.inner.process_names()
    }

    /// Returns the number of messages sent by the process.
    pub fn sent_message_count(&self, proc: &str) -> u64 {
        self.inner.sent_message_count(proc)
    }

    /// Returns the number of messages received by the process.
    pub fn received_message_count(&self, proc: &str) -> u64 {
        self.inner.received_message_count(proc)
    }

    // Simulation -----------------------------------------------------

    /// Steps through the simulation until there are no pending events left.
    pub fn step_until_no_events(&mut self) {
        self.inner.step_until_no_events()
    }

    /// Perform `steps` steps through the simulation.
    pub fn make_steps(&mut self, steps: u32) {
        for _ in 0..steps {
            let something_happen = self.step();
            if !something_happen {
                break;
            }
        }
    }

    /// Perform single step through the simulation.
    pub fn step(&mut self) -> bool {
        self.inner.step()
    }
}
