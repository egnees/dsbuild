//! Definition of manager of nodes in virtual system.
//!
//! The main purpose of node manager is to map node names to their network addressed,
//! and also perform mapping from [`DSLab MP`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)
//! unique process names to their addresses.

use std::collections::{HashMap, HashSet};

use crate::common::process::Address;

/// Represents node manager.
///
/// WARNING: Node manager does not permit nodes and processes with names, contains `/`.
/// Allowing this can potentially lead to problems with mapping to and from
/// [`DSLab MP`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)
/// process names in form of `node_name/process_name`.
#[derive(Default)]
pub struct NodeManager {
    name_to_address: HashMap<String, Address>,
    address_to_name: HashMap<Address, String>,
    node_processes: HashMap<String, HashSet<String>>,
}

impl NodeManager {
    /// Add new node to the manager.
    ///
    /// # Returns
    ///
    /// - Error in case of node with such `name` or with such `host` and `port` already exists.
    /// - Error in case of node `name` is empty or contains `/` character.
    /// - Ok in any other case.
    pub fn add_node(&mut self, name: String, host: String, port: u16) -> Result<(), String> {
        Self::check_name(&name)?;

        let address = Address::new_node_address(host, port);
        if self.name_to_address.contains_key(&name) {
            Err(format!("Node with name {} already exists.", &name))
        } else {
            // Add node to mapping.
            self.name_to_address.insert(name.clone(), address.clone());

            // Add node to the reverse mapping.
            let addr_to_name_opt = self.address_to_name.insert(address.clone(), name.clone());

            // Panic in case node with same address already exists in hashmap,
            // which means not synchronized mappings and implementation error.
            if addr_to_name_opt.is_some() {
                panic!(
                    "Incorrect implementation of NodeManager: 
                    mappings address_to_name and name_to_address are not synchronized"
                );
            }

            // Add node to the node process mapping.
            let node_processes_opt = self.node_processes.insert(name, HashSet::new());

            // Panic in case node with same address already exists in hashmap,
            // which means not synchronized mappings and implementation error.
            if node_processes_opt.is_some() {
                panic!(
                    "Incorrect implementation of NodeManager: 
                    mappings address_to_name and node_process are not synchronized"
                );
            }

            Ok(())
        }
    }

    /// Add new process to the node.
    ///
    /// # Returns
    ///
    /// - Error in case of node with such `node_name` does not exists.
    /// - Error in case of process with such `process_name` already exists on the node.
    /// - Error in case of `node_name` is empty or contains `/` character.
    /// - Error in case of `process_name` is empty or contains `/` character.
    /// - Ok with [`address`][`Address`] of the added process in any other case.
    pub fn add_process_to_node(
        &mut self,
        node_name: String,
        process_name: String,
    ) -> Result<Address, String> {
        Self::check_name(&node_name)?;
        Self::check_name(&process_name)?;

        if let Some(node_processes) = self.node_processes.get_mut(&node_name) {
            if node_processes.contains(&process_name) {
                Err(format!(
                    "Node {} already contains process {}.",
                    &node_name, &process_name
                ))
            } else {
                let insert_result = node_processes.insert(process_name.clone());
                assert!(insert_result, "Incorrect implementation of NodeManager.");

                // Get process address.
                let mut node_address = self.name_to_address.get(&node_name).expect("Incorrect implementation of NodeManager: name_to_address is not synchronized with node_processes.").clone();
                node_address.process_name = process_name;

                // Return process address.
                Ok(node_address)
            }
        } else {
            Err(format!("Node with name {} does not exist.", &node_name))
        }
    }

    /// Removed all processes from the node.
    pub fn clear_node(&mut self, node_name: &str) {
        self.node_processes.get_mut(node_name).unwrap().clear();
    }

    /// Map process by it's address into the view `node_name/process_name`,
    /// in which process can be added to the [`dslab_mp`](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)).
    ///
    /// # Returns
    ///
    /// - Error in case node with such [`address`][`Address`] is not exists.
    /// - Error in case process with such [`name`][Address::process_name`] is not exists on the node.
    /// - Ok with mapped process name in case of success.
    pub fn get_full_process_name(&self, address: &Address) -> Result<String, String> {
        // Check such node with such address exists.
        let node_address = Address::new_node_address(address.host.clone(), address.port);
        if !self.address_to_name.contains_key(&node_address) {
            return Err(format!(
                "Node with address {:?} does not exists.",
                &node_address
            ));
        }

        // Get node name.
        let node_name = self
            .address_to_name
            .get(&node_address)
            .expect("Incorrect implementation of NodeManager.");

        // Get node processes.
        let processes = self.node_processes
            .get(node_name)
            .expect("Incorrect implementation of NodeManager: mappings address_to_name and node_processes not synchronized.");

        // Check process with such name exists on the node.
        let process_name = processes.get(&address.process_name).ok_or(format!(
            "Process with name {} not found on node {}.",
            &address.process_name, &node_name
        ))?;

        // Map to the view of node_name/process_name.
        Ok(format!("{}/{}", node_name, process_name))
    }

    /// Map full process name, potentially received from [`NodeManager::get_full_process_name()`],
    /// to the [`process address`][`Address`].
    ///
    /// # Returns
    ///
    /// - Error in case process_name is not in the view of `node_name/process_name`.
    /// - Error if the node with such `node_name` does not exists.
    /// - Error if the process with such `process_name` does not exists on the node.
    /// - Ok with mapped [`process address`][`Address`] in case of success.
    pub fn get_process_address(&self, process_name: &str) -> Result<Address, String> {
        // Check if process_name is in the view of node_name/process_name.
        if let Some((node_name, process_name)) = process_name.split_once('/') {
            // Check if name of node or name of process is empty.
            if node_name.is_empty() || process_name.is_empty() {
                Err(format!(
                    "Invalid process name: {}, name of node and name of process can not be empty",
                    process_name
                ))
            } else {
                // Get node name by it's address.
                let node_address = self
                    .name_to_address
                    .get(node_name)
                    .ok_or(format!("Node with name {} not found.", node_name))?;

                // Fill process address.
                let mut process_address = node_address.clone();
                process_address.process_name = process_name.to_owned();

                // Return process address.
                Ok(process_address)
            }
        } else {
            Err(format!("Invalid process name: {}.", process_name))
        }
    }

    /// Check if node with such `node_name` exists.
    pub fn check_node_exists(&self, node_name: &str) -> bool {
        self.node_processes.contains_key(node_name)
    }

    /// Check if process with such `process_name` exists on the node with such `node_name`.
    pub fn check_process_exists(&self, node_name: &str, process_name: &str) -> bool {
        if !self.check_node_exists(node_name) {
            return false;
        }

        let node_processes = self.node_processes
            .get(node_name)
            .expect("Incorrect implementation of node manager: mappings address_to_name and node_processes not synchronized.");

        node_processes.contains(process_name)
    }

    /// Returns full name of the process `process_name` located on the node `node_name`.
    ///
    /// # Returns
    /// - Error in case process with such `process_name` does not exists on the node with such `node_name`.
    /// - Ok with full process name in case of success.
    pub fn construct_full_process_name(
        &self,
        process_name: &str,
        node_name: &str,
    ) -> Result<String, String> {
        if !self.check_process_exists(node_name, process_name) {
            return Err(format!(
                "Process with name {} not found on node {}.",
                process_name, node_name
            ));
        }

        Ok(format!("{}/{}", node_name, process_name))
    }

    /// Check if the `name` is valid.
    ///
    /// # Returns
    ///
    /// - Error in case of `name` is empty or contains `/` character.
    /// - Ok in any other case.
    ///
    /// Such behavior is required to allow determinate mapping
    /// from node_name/proc_name view into [`process address`][`Address`].
    fn check_name(name: &str) -> Result<(), String> {
        if name.is_empty() || name.contains('/') {
            Err("Node name can not be empty or contain '/'.".to_owned())
        } else {
            Ok(())
        }
    }
}
