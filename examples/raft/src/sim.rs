use std::collections::{HashMap, VecDeque};

use dsbuild::{Address, Message, Sim};

use crate::{
    cmd::{Command, CommandId, CommandReply, CommandType, ValueType},
    local::{
        InitializeRequest, InitializeResponse, LocalResponse, LocalResponseType, ReadValueRequest,
        INITIALIZE_RESPONSE, LOCAL_RESPONSE,
    },
    proc::RaftProcess,
    role::Role,
    state::{StateInfo, STATE_INFO},
};

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Clone)]
pub struct ProcessInfo {
    pub cmd_seq_num: usize,
    node: usize,
    last_state_info: Option<StateInfo>,
    local_msgs: VecDeque<LocalResponse>,
    initialized: bool,
    shutdown: bool,
}

//////////////////////////////////////////////////////////////////////////////////////////

impl ProcessInfo {
    fn new(node: usize) -> Self {
        Self {
            cmd_seq_num: 0,
            node,
            last_state_info: Default::default(),
            local_msgs: Default::default(),
            initialized: false,
            shutdown: false,
        }
    }

    fn on_local_message(&mut self, message: Message) {
        match message.get_tip().as_str() {
            STATE_INFO => {
                self.last_state_info.replace(message.into());
            }
            LOCAL_RESPONSE => {
                if self.initialized {
                    self.local_msgs.push_back(message.into())
                }
            }
            INITIALIZE_RESPONSE => {
                self.initialized = true;
                self.cmd_seq_num = InitializeResponse::from(message).seq_num;
            }
            _ => panic!("unexpected local message tip"),
        }
    }

    fn next_command_id(&mut self) -> CommandId {
        self.cmd_seq_num += 1;
        CommandId(self.node, self.cmd_seq_num)
    }

    fn reset(&mut self) {
        self.local_msgs.clear();
        self.initialized = false;
        self.cmd_seq_num = 0;
        self.last_state_info.take();
    }

    fn shutdown(&mut self) {
        self.shutdown = true;
        self.reset();
    }

    fn rerun(&mut self) {
        self.shutdown = false;
        self.reset();
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to get next local message
    pub fn peek_next_local(&self) -> Option<&LocalResponse> {
        self.local_msgs.front()
    }

    /// Allows to pop next local message
    pub fn pop_next_local(&mut self) -> Option<LocalResponse> {
        self.local_msgs.pop_front()
    }

    /// Pop next local message with expectation
    pub fn pop_local_and_expect(&mut self, response: LocalResponse) {
        let exists = self.pop_next_local().unwrap();
        assert_eq!(exists, response);
    }

    pub fn pop_local_and_expect_command_id(&mut self, id: CommandId) {
        let response = self.pop_next_local().unwrap();
        assert_eq!(response.request_id, id);
    }

    /// Pop next local mesage with type expectation
    pub fn pop_local_and_expect_type(&mut self, response_type: LocalResponseType) {
        let exists = self.pop_next_local().unwrap();
        assert_eq!(exists.tp, response_type);
    }

    /// Pop next local and sure its read value response
    pub fn pop_local_and_assure_read_value(&mut self) -> Option<ValueType> {
        let next_local = self.pop_next_local().unwrap();
        match next_local.tp {
            LocalResponseType::ReadValue(value) => value,
            _ => panic!("unexpected local response"),
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    pub fn clear_locals(&mut self) {
        self.local_msgs.clear();
    }

    pub fn locals_len(&mut self) -> usize {
        self.local_msgs.len()
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    pub fn pop_local_and_expect_unavailable(&mut self) {
        self.pop_local_and_expect_type(LocalResponseType::Unavailable());
    }

    pub fn pop_local_and_expect_read_value(&mut self, value: Option<&str>) {
        self.pop_local_and_expect_type(LocalResponseType::ReadValue(value.map(|s| s.to_owned())));
    }

    pub fn pop_local_and_expect_redirected_to(
        &mut self,
        leader: usize,
        min_commit_index: Option<i64>,
    ) {
        self.pop_local_and_expect_type(LocalResponseType::RedirectedTo(leader, min_commit_index));
    }

    pub fn pop_local_and_expect_command_reply(&mut self, status: u16) {
        let exists = self.pop_next_local().unwrap().tp;
        assert!(
            matches!(exists, LocalResponseType::Command(CommandReply {status: s, ..}) if s == status)
        )
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    pub fn expect_no_local(&mut self) {
        assert!(self.local_msgs.is_empty());
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

pub struct SimWrapper {
    sim: Sim,
    node_cnt: usize,
    proc_info: Vec<ProcessInfo>,
}

//////////////////////////////////////////////////////////////////////////////////////////

impl SimWrapper {
    pub fn new(seed: u64, node_cnt: usize) -> Self {
        let sim = Sim::new(seed);
        sim.set_network_delays(0.15 / 2. - 0.025, 0.15 / 2. + 0.025);

        let mut sim_wrapper = Self {
            sim,
            node_cnt,
            proc_info: (0..node_cnt).map(ProcessInfo::new).collect(),
        };

        for node in 0..node_cnt {
            sim_wrapper.add_node(node);
            sim_wrapper.add_proccess_to_node(node);
        }

        sim_wrapper
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    fn add_node(&mut self, node: usize) {
        let node_name = Self::node_name(node);
        let port = 123;
        let storage = 1 << 15;
        self.sim
            .add_node_with_storage(&node_name, &node_name, port, storage);
    }

    fn add_proccess_to_node(&mut self, node: usize) {
        let nodes = Self::all_processes(self.node_cnt).collect::<Vec<_>>();
        let node_name = Self::node_name(node);
        let process_name = Self::process_name(node);
        let process = RaftProcess::new(nodes, node, 0.15);
        self.sim.add_process(&process_name, process, &node_name);
    }

    fn node_name(node: usize) -> String {
        format!("node_{}", node)
    }

    fn process_name(proc: usize) -> String {
        format!("process_{}", proc)
    }

    fn process_addr(node: usize) -> Address {
        Address {
            host: Self::node_name(node),
            port: 123,
            process_name: Self::process_name(node),
        }
    }

    fn all_processes(node_cnt: usize) -> impl Iterator<Item = Address> {
        (0..node_cnt).map(Self::process_addr)
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to send [`InitializeRequest`] for all processes
    pub fn send_init_for_all(&mut self) {
        for node in 0..self.node_cnt {
            let proc = Self::process_name(node);
            let node = Self::node_name(node);
            self.sim
                .send_local_message(&proc, &node, InitializeRequest {}.into());
        }
    }

    /// Allows to shutdown node
    pub fn shutdown_node(&mut self, node: usize) {
        self.proc_info[node].shutdown();
        let node_name = Self::node_name(node);
        self.sim.shutdown_node(&node_name);
    }

    /// Allows to run previously shut down node
    pub fn rerun_node(&mut self, node: usize) {
        let node_name = Self::node_name(node);
        self.sim.rerun_node(&node_name);

        self.add_proccess_to_node(node);

        let process_name = Self::process_name(node);
        self.sim
            .send_local_message(&process_name, &node_name, InitializeRequest {}.into());

        self.proc_info[node].rerun();
    }

    /// Allows to get info about process hosted on node
    pub fn process(&mut self, node: usize) -> &mut ProcessInfo {
        &mut self.proc_info[node]
    }

    /// Allows to make steps
    pub fn make_steps(&mut self, steps: usize) {
        (0..steps).for_each(|_| self.make_step());
    }

    /// Allows to make step
    pub fn make_step(&mut self) {
        self.sim.step();
        for proc in 0..self.node_cnt {
            if self.proc_info[proc].shutdown {
                continue;
            }

            let proc_name = Self::process_name(proc);
            let node_name = Self::node_name(proc);
            if let Some(messages) = self.sim.read_local_messages(&proc_name, &node_name) {
                messages
                    .into_iter()
                    .for_each(|message| self.proc_info[proc].on_local_message(message));
            }
        }
    }

    /// Allows to step until process will receive local message
    pub fn make_steps_until_local_message(&mut self, proc: usize) {
        while self.proc_info[proc].local_msgs.is_empty() {
            self.make_step();
        }
    }

    /// Make steps until reply with specified command id will be returned
    /// All preceeding commands will be removed
    pub fn make_steps_until_response_id(&mut self, proc: usize, reply_on: CommandId) {
        while self.proc_info[proc]
            .peek_next_local()
            .map(|reply| reply.request_id)
            != Some(reply_on)
        {
            self.proc_info[proc].pop_next_local();
            self.make_step();
        }
    }

    /// Allows to step until process initialized
    pub fn make_steps_until_initialized(&mut self, proc: usize) {
        while !self.proc_info[proc].initialized {
            self.make_step();
        }
    }

    /// Allows to step until all processes initialized
    pub fn make_steps_until_all_initialized(&mut self) {
        while (0..self.node_cnt).all(|proc| self.proc_info[proc].initialized) {
            self.make_step();
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to send command to process
    pub fn send_command(&mut self, proc: usize, cmd: CommandType) -> CommandId {
        let command_id = self.proc_info[proc].next_command_id();
        let process_name = Self::process_name(proc);
        let node_name = Self::node_name(proc);
        let command = Command::new(cmd, command_id);
        self.sim
            .send_local_message(&process_name, &node_name, command.into());
        command_id
    }

    pub fn send_read_request(&mut self, proc: usize, key: &str) -> CommandId {
        let request_id = self.proc_info[proc].next_command_id();
        let process_name = Self::process_name(proc);
        let node_name = Self::node_name(proc);
        let command = ReadValueRequest {
            key: key.to_owned(),
            request_id,
            min_commit_id: None,
        };
        self.sim
            .send_local_message(&process_name, &node_name, command.into());
        request_id
    }

    pub fn send_read_request_with_commit_idx(
        &mut self,
        proc: usize,
        key: &str,
        commit_idx: i64,
    ) -> CommandId {
        let request_id = self.proc_info[proc].next_command_id();
        let process_name = Self::process_name(proc);
        let node_name = Self::node_name(proc);
        let command = ReadValueRequest {
            key: key.to_owned(),
            request_id,
            min_commit_id: Some(commit_idx),
        };
        self.sim
            .send_local_message(&process_name, &node_name, command.into());
        request_id
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to get current leader
    pub fn current_leader(&self) -> Option<usize> {
        let mut leader: Option<(usize, usize)> = None; // (term, leader)
        for proc in 0..self.node_cnt {
            let proc_info = &self.proc_info[proc].last_state_info;
            if let Some(state_info) = proc_info {
                if let Role::Leader(_) = &state_info.role {
                    match leader {
                        None => leader = Some((state_info.current_term, proc)),
                        Some((_, term)) => {
                            if term < state_info.current_term {
                                leader = Some((state_info.current_term, proc))
                            }
                        }
                    }
                }
            }
        }
        leader.map(|(_, leader)| leader)
    }

    /// Allows to get current term
    pub fn current_term(&self) -> Option<usize> {
        let mut term_info = HashMap::new();
        for proc in 0..self.node_cnt {
            let proc_info = &self.proc_info[proc].last_state_info;
            if let Some(state_info) = proc_info {
                let old_value = term_info
                    .get(&state_info.current_term)
                    .copied()
                    .unwrap_or(0usize);
                term_info.insert(state_info.current_term, old_value + 1);
            }
        }
        let majority = self.node_cnt / 2 + 1;
        term_info
            .iter()
            .filter(|(_k, v)| (**v) >= majority)
            .map(|(k, _v)| *k)
            .last()
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to split network
    pub fn split_network(&mut self, part: &[usize]) {
        let group1 = part
            .iter()
            .copied()
            .map(Self::node_name)
            .collect::<Vec<_>>();

        let group1_ref = group1.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        let group2 = (0..self.node_cnt)
            .filter(|node| !part.contains(node))
            .map(Self::node_name)
            .collect::<Vec<_>>();

        let group2_ref = group2.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        self.sim
            .split_network(group1_ref.as_slice(), group2_ref.as_slice());
    }

    /// Allows to repair remove network split
    pub fn repair_network(&mut self) {
        for node in 0..self.node_cnt {
            let node = Self::node_name(node);
            self.sim.connect_node_to_network(&node);
        }
    }
}
