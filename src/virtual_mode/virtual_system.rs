//! Definition of [`VirtualSystem`].

use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::{Arc, RwLock},
};

use dslab_mp::{
    message::Message as SimulationMessage, network::Network, node::Node,
    system::System as Simulation,
};

use super::{node_manager::NodeManager, process_wrapper::VirtualProcessWrapper};
use crate::common::process::{Process, ProcessWrapper};

/// Represents virtual system, which is responsible
/// for interacting with [`user-processes`][`Process`],
/// simulating time, network, and other [OS](https://en.wikipedia.org/wiki/Operating_system) features.
///
/// [`VirtualSystem`] uses [DSLab MP](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)
/// framework for simulation of network, time, etc.
pub struct VirtualSystem {
    inner: Simulation,
    node_manager: Rc<RefCell<NodeManager>>,
}

impl VirtualSystem {
    /// Creates new [`VirtualSystem`] with provided `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            inner: Simulation::new(seed),
            node_manager: Rc::new(RefCell::new(NodeManager::default())),
        }
    }

    // Network ---------------------------------------------------

    /// Returns a mutable reference to network.
    pub fn network(&self) -> RefMut<Network> {
        self.inner.network()
    }

    // Node ------------------------------------------------------

    /// Adds a node to the simulation.
    /// Note that node names must be unique and does not contain `/` symbol.
    ///
    /// # Panics
    ///
    /// - In case node with such `name` already exists.
    /// - In case `name` is empty or contains `/` character.
    pub fn add_node(&mut self, name: &str, host: &str, port: u16) {
        // Add node to the node manager.
        self.node_manager
            .borrow_mut()
            .add_node(name.to_owned(), host.to_owned(), port)
            .unwrap();

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

    /// Returns a mutable reference to the node.
    /// 
    /// Can not make method public because
    /// process names on dslab nodes are not the same as in the framework.  
    fn get_mut_node(&self, name: &str) -> Option<RefMut<Node>> {
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
    /// - If node with such name `node_name` does not exists.
    /// - If process with such `process name` is already exists on the node with `node_name`.
    /// - If process name or node name is empty or contains `/` symbol.
    pub fn add_process<P: Process + Clone + 'static>(
        &mut self,
        process_name: &str,
        process: P,
        node_name: &str,
    ) -> ProcessWrapper<P> {
        // Add process to the node manager.
        let process_address = self
            .node_manager
            .borrow_mut()
            .add_process_to_node(node_name.to_owned(), process_name.to_owned())
            .unwrap();

        // Get full process name.
        let full_process_name = self
            .node_manager
            .borrow()
            .get_full_process_name(&process_address)
            .expect("Implementation error: can not get full name of registered process.");

        // Configure process ref.
        let process_ref = Arc::new(RwLock::new(process));

        // Configure virtual process wrapper.
        let node_manager_ref = self.node_manager.clone();
        let virtual_proc_wrapper =
            VirtualProcessWrapper::new(process_address, process_ref.clone(), node_manager_ref);

        // Configure wrapper to the dslab.
        let process_wrapper = ProcessWrapper { process_ref };
        let boxed_wrapper = Box::new(virtual_proc_wrapper);

        // Add virtual process wrapper to the dslab.
        self.inner
            .add_process(&full_process_name, boxed_wrapper, node_name);

        // Return process wrapper to user.
        process_wrapper
    }

    /// Start process
    /// Call on_start method of process
    pub fn start(&mut self, proc: &str, node: &str) {
        let full_process_name = self.node_manager.borrow().construct_full_process_name(proc, node).unwrap();
        if let Some(mut node_ref) = self.get_mut_node(node) {
            node_ref.send_local_message(full_process_name, SimulationMessage::new("START", ""));
        }
    }

    /// Returns the names of all processes in the system.
    pub fn process_names(&self) -> Vec<String> {
        self.inner.process_names()
    }

    /// Returns the number of messages sent by the process.
    pub fn sent_message_count(&self, proc: &str, node: &str) -> u64 {
        let full_process_name = self.node_manager.borrow().construct_full_process_name(proc, node).unwrap();
        self.inner.sent_message_count(&full_process_name)
    }

    /// Returns the number of messages received by the process.
    pub fn received_message_count(&self, proc: &str, node: &str) -> u64 {
        let full_process_name = self.node_manager.borrow().construct_full_process_name(proc, node).unwrap();
        self.inner.received_message_count(&full_process_name)
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
