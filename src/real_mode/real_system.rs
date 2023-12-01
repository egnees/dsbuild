//! Implementation of real mode system runner.

use crate::common::process::Process;
use crate::real_mode::process_runner::{ProcessRunner, RunConfig};

/// Specifies real mode system.
pub struct RealSystem {
    // Here can be stored some meta information specified by user.
    // For examples, maximum number of threads or maximum size of UDP datagram.
}

impl RealSystem {
    /// Create new instance of System.
    pub fn new() -> Self {
        Self {}
    }

    /// Run user process, which type must implement trait [Process](Process).
    /// Returns ownership of the passed user process `proc` and result of process run.
    /// After method returns, state of the system is invalidated.
    //
    /// * `proc` - instantiated user process.
    /// * `host` - IPv4 or IPv6 address identifies running process in the network.
    pub fn run_process<'a, P: Process>(&self, proc: &'a mut P, host: &str) -> Result<(), String> {
        let config = RunConfig {
            host: host.to_string(),
        };
        ProcessRunner::new(config)?.run(proc)
    }
}
