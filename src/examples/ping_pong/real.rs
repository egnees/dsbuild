//! Real-time usage of the Ping-Pong ecosystem.
//!
//! Creates Ping and Pong processes in two different threads and make them communicate with each other.

use std::thread::{self, sleep};
use std::time::Duration;

use crate::common::process::Address;
use crate::examples::ping_pong::{pinger, ponger};
use crate::{RealSystem, RealSystemConfig};

/// Runs real system with specified number of ping-pong iterations.
pub fn run_real(need_cycles: u32) {
    // Pinger and Ponger name.
    const PONGER_NAME: &str = "Ponger";
    const PINGER_NAME: &str = "Pinger";

    // Process ports.
    const PINGER_PORT: u16 = 10091;
    const PONGER_PORT: u16 = 10092;

    // Process addresses.
    let ponger_addr: Address =
        Address::new("127.0.0.1".to_string(), PONGER_PORT, PONGER_NAME.to_owned());

    // Spawn pinger.
    let pinger_thread = thread::spawn(move || {
        // Create pinger process with 0.1 second retry delay and 100 need pings before stop.
        let pinger = pinger::create_pinger(0.1, ponger_addr, need_cycles);

        // Create config, which will help to create system.
        let config = RealSystemConfig::default("127.0.0.1".to_owned(), PINGER_PORT)
            .expect("Can not create config");

        // Create system by config.
        let mut system = RealSystem::new(config).expect("Can not create system");

        // Add pinger to the system and get process wrapper.
        let pinger_wrapper = system
            .add_process(PINGER_NAME, pinger)
            .expect("Can not add pinger");

        // Run the system.
        system
            .run()
            .expect("Error in the process of running the system");

        // Check pinger is stopped.
        assert!(pinger_wrapper.read().is_stopped());
    });

    // Spawn ponger.
    let ponger_thread = thread::spawn(|| {
        // Sleep to see how pinger process works when can not communicate with ponger.
        sleep(Duration::from_secs_f64(0.1));

        // Create ponger process with 1 second inactivity window.
        let ponger = ponger::create_ponger(3.0);

        // Create config, which will help to create system.
        let config = RealSystemConfig::default("127.0.0.1".to_owned(), PONGER_PORT)
            .expect("Can not create config");

        // Create system by config.
        let mut system = RealSystem::new(config).expect("Can not create system");

        // Add pinger to the system and get process wrapper.
        let ponger_wrapper = system
            .add_process(PONGER_NAME, ponger)
            .expect("Can not add ponger");

        // Run the system.
        system
            .run()
            .expect("Error in the process of running the system");

        // Check ponger is stopped.
        assert!(ponger_wrapper.read().is_stopped());
    });

    // Wait for pinger process and ponger process to finish.
    pinger_thread.join().unwrap();
    ponger_thread.join().unwrap();
}
