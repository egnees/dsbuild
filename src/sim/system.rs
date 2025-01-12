//! Simulation.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

use dslab_async_mp::system::System as DSLabSimulation;

use super::{node::NodeManager, process::VirtualProcessWrapper};

use crate::{
    common::process::{Process, ProcessWrapper},
    Message,
};

////////////////////////////////////////////////////////////////////////////////

/// Represensts simulation of real world system environment: nodes, network, time and file system.
///
/// Simulation is event-driven: in every moment there are pending events, ordered by time.
/// Every event can cause other events and execute corresponding callback of the process-receiver.
/// To make system to process event chain, user should call the following methods of simulation:
/// [`step`][Sim::step], [`make_steps`][Sim::make_steps], [`step_until_no_events`][Sim::step_until_no_events].
///
/// User can [add nodes][Sim::add_node] to simulation with specified configuration,
/// connect and disconnect them from network.
/// User can [add processes][Sim::add_process] on some nodes and then communicate with them by
/// [sending][Sim::send_local_message] and [reading][Sim::read_local_messages] local messages.
/// Also, user can [crash][Sim::crash_node] nodes and [recover][Sim::recover_node] them.
///
/// Simulation allows to configure network settings.
/// For example, user can set [delays][Sim::set_network_delays] of the network and its
/// [drop-rate][Sim::set_network_drop_rate].
pub struct Sim {
    inner: DSLabSimulation,
    node_manager: Rc<RefCell<NodeManager>>,
}

impl Sim {
    /// Create new simulation with provided seed.
    pub fn new(seed: u64) -> Self {
        let inner = DSLabSimulation::new(seed);
        inner.network().set_corrupt_rate(0.0);
        inner.network().set_drop_rate(0.0);
        inner.network().set_delays(0.5, 1.0);
        Self {
            inner,
            node_manager: Rc::new(RefCell::new(NodeManager::default())),
        }
    }

    ////////////////////////////////////////////////////////////////////////////////
    // Network
    ////////////////////////////////////////////////////////////////////////////////

    /// Set the fixed network delay.
    pub fn set_network_delay(&self, delay: f64) {
        self.inner.network().set_delay(delay)
    }

    /// Set the minimum and maximum network delays.
    pub fn set_network_delays(&self, min_delay: f64, max_delay: f64) {
        self.inner.network().set_delays(min_delay, max_delay)
    }

    /// Set drop rate of the network.
    pub fn set_network_drop_rate(&self, drop_rate: f64) {
        self.inner.network().set_drop_rate(drop_rate)
    }

    /// Connect node to the network
    pub fn connect_node_to_network(&self, node: &str) {
        self.inner.network().connect_node(node)
    }

    /// Disconnect node from the network
    pub fn disconnect_node_from_network(&self, node: &str) {
        self.inner.network().disconnect_node(node)
    }

    /// Allows to disable pairwise connections between groups.
    pub fn split_network(&self, group1: &[&str], group2: &[&str]) {
        self.inner.network().make_partition(group1, group2)
    }

    ////////////////////////////////////////////////////////////////////////////////
    // Node
    ////////////////////////////////////////////////////////////////////////////////

    /// Add node to simulation.
    ///
    /// Node names must be unique and not contain `/` symbol.
    ///
    /// # Panics
    ///
    /// - In case node with such `name` already exists.
    /// - In case `name` is empty or contains `/` character.
    pub fn add_node(&mut self, name: &str, host: &str, port: u16) {
        self.add_node_with_storage(name, host, port, 0);
    }

    /// Add node with specified storage capacity to simulation.
    ///
    /// Note that node names must be unique and not contain `/` symbol.
    ///
    /// # Panics
    ///
    /// - In case node with such `name` already exists.
    /// - In case `name` is empty or contains `/` character.
    pub fn add_node_with_storage(
        &mut self,
        name: &str,
        host: &str,
        port: u16,
        storage_capacity: usize,
    ) {
        // Add node to the node manager.
        self.node_manager
            .borrow_mut()
            .add_node(name.to_owned(), host.to_owned(), port)
            .unwrap();

        self.inner.add_node_with_storage(name, storage_capacity);
    }

    /// Crashes the specified node and its storage.
    ///
    /// All pending events created by the node will be discarded.
    /// The undelivered messages sent by the node will be dropped.
    /// All pending and future events destined to the node will be discarded.
    ///
    /// Processes running on the node are not cleared to allow working
    /// with processes after the crash (i.e. examine event log).
    pub fn crash_node(&mut self, node_name: &str) {
        self.inner.crash_node(node_name);
        self.node_manager.borrow_mut().clear_node(node_name);
    }

    /// Recovers the previously crashed node.
    ///
    /// Processes running on the node before the crash are cleared.
    /// The delivery of events to the node is enabled.
    pub fn recover_node(&mut self, node_name: &str) {
        self.inner.recover_node(node_name);
    }

    /// Shutdowns the specified node with saving storage.
    pub fn shutdown_node(&mut self, node_name: &str) {
        self.inner.shutdown_node(node_name);
        self.node_manager.borrow_mut().clear_node(node_name);
    }

    /// Reruns previously shut node.
    pub fn rerun_node(&mut self, node_name: &str) {
        self.inner.rerun_node(node_name);
    }

    /// Checks if the node is crashed.
    pub fn is_node_crashed(&self, node: &str) -> bool {
        self.inner.node_is_crashed(node)
    }

    // Process ------------------------------------------------------

    /// Add process.
    ///
    /// # Panics
    ///
    /// - If node with such name `node_name` does not exists.
    /// - If process with such `process name` already exists on the node with name `node_name`.
    /// - If `process name` or `node name` is empty or contains `/` symbol.
    pub fn add_process<P: Process + 'static>(
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
            VirtualProcessWrapper::new(process_ref.clone(), node_manager_ref);

        // Configure wrapper to the dslab.
        let process_wrapper = ProcessWrapper { process_ref };
        let boxed_wrapper = Box::new(virtual_proc_wrapper);

        // Add virtual process wrapper to the dslab.
        self.inner
            .add_process(&full_process_name, boxed_wrapper, node_name);

        // Return process wrapper to user.
        process_wrapper
    }

    /// Get names of all processes in the system.
    pub fn process_names(&self) -> Vec<String> {
        self.inner.process_names()
    }

    /// Extracts and returns local messages, produced by the process.
    pub fn read_local_messages(&mut self, proc: &str, node: &str) -> Option<Vec<Message>> {
        let full_process_name = self
            .node_manager
            .borrow()
            .construct_full_process_name(proc, node)
            .unwrap();

        self.inner
            .read_local_messages(&full_process_name)
            .map(|messages| messages.into_iter().map(|msg| msg.into()).collect())
    }

    /// Send local message to the process.
    pub fn send_local_message(&mut self, proc: &str, node: &str, msg: Message) {
        let full_process_name = self
            .node_manager
            .borrow()
            .construct_full_process_name(proc, node)
            .unwrap();

        self.inner
            .send_local_message(&full_process_name, msg.into());
    }

    /// Returns the number of messages sent by the process.
    pub fn sent_message_count(&self, proc: &str, node: &str) -> u64 {
        let full_process_name = self
            .node_manager
            .borrow()
            .construct_full_process_name(proc, node)
            .unwrap();
        self.inner.sent_message_count(&full_process_name)
    }

    /// Returns the number of messages received by the process.
    pub fn received_message_count(&self, proc: &str, node: &str) -> u64 {
        let full_process_name = self
            .node_manager
            .borrow()
            .construct_full_process_name(proc, node)
            .unwrap();
        self.inner.received_message_count(&full_process_name)
    }

    /// Steps through the simulation until there are no pending events left.
    pub fn step_until_no_events(&mut self) {
        self.inner.step_until_no_events()
    }

    /// Steps through the simulation until there are no local messages.
    pub fn step_until_local_message(
        &mut self,
        proc: &str,
        node: &str,
    ) -> Result<Vec<Message>, String> {
        let full_process_name = self
            .node_manager
            .borrow()
            .construct_full_process_name(proc, node)
            .unwrap();

        self.inner
            .step_until_local_message(&full_process_name)
            .map_err(|str| str.to_owned())
            .map(|v| v.into_iter().map(|m| m.into()).collect())
    }

    /// Perform specified number of steps through the simulation.
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
