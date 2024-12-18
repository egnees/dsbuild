#[allow(dead_code)]
pub enum SimulationCommand {
    /// Get current leader
    Leader(),

    /// Get current term
    Term(),

    /// Shutdown node
    Shutdown(usize),

    /// Reboot node
    Rerun(usize),

    /// Get response on command,
    Get(raft::cmd::CommandId),

    /// Send create request
    Create(usize, raft::cmd::KeyType),

    /// Send update request
    Update(usize, raft::cmd::KeyType, raft::cmd::ValueType),

    /// Send delete request
    Delete(usize, raft::cmd::KeyType),

    /// Send cas request
    Cas(
        usize,
        raft::cmd::KeyType,
        raft::cmd::KeyType,
        raft::cmd::ValueType,
    ),

    /// Send read request
    Read(usize, raft::cmd::KeyType),

    /// Make simulation steps
    Steps(usize),

    /// Disconnect node from network
    Disconnect(usize),

    /// Repair network
    Repair,

    /// Get help
    Help,
}

impl SimulationCommand {
    #[allow(dead_code)]
    pub fn from_str(_s: &str) {
        unimplemented!("to be implemented")
    }
}
