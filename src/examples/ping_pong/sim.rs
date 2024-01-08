//! Simulation of real-time usage of the Ping-Pong ecosystem.
//!
//! Creates Ping and Pong processes, add them to system, and make them communicate with each other.

use crate::examples::ping_pong::{pinger, ponger};
use crate::VirtualSystem;

/// Runs simulation with specified number of ping-pong iterations.
pub fn run_sim(need_cycles: u32) {
    // Pinger and names.
    const PONGER_NAME: &str = "Ponger";
    const PINGER_NAME: &str = "Pinger";

    // Process nodes.
    const PONGER_NODE: &str = "Ponger node";
    const PINGER_NODE: &str = "Pinger node";

    // Create simulation with specified seed.
    let mut sim = VirtualSystem::new(12345);

    // Configure simulation network.
    sim.network().set_drop_rate(0.65);
    sim.network().set_delays(0.025, 0.375);

    // Add pinger node to the simulation.
    sim.add_node(PINGER_NODE);

    // Add ponger node to the simulation.
    sim.add_node(PONGER_NODE);

    // Connect both nodes to the network.
    sim.network().connect_node(PINGER_NAME);
    sim.network().connect_node(PONGER_NAME);

    // Add pinger to the node.
    let pinger = pinger::create_pinger(0.1, PONGER_NAME.to_owned(), need_cycles);
    let pinger_wrapper = sim.add_process(PINGER_NAME, pinger, PINGER_NODE);

    // Add ponger to the node.
    let ponger = ponger::create_ponger(3.0);
    let ponger_wrapper = sim.add_process(PONGER_NAME, ponger, PONGER_NODE);

    // Try to start pinger process.
    sim.start(PINGER_NAME, "Pinger node");

    // Check pinger is started.
    assert!(pinger_wrapper.read().is_started());

    // Check ponger is not started yet.
    assert!(!ponger_wrapper.read().is_started());

    // Make two steps.
    // Pinger will send ponger message, but ponger will ignore it.
    sim.make_steps(2);

    // Now pinger must try to send more messages,
    // but ponger is not started yet.
    sim.make_steps(2);

    // Now start the ponger process.
    sim.start(PONGER_NAME, PONGER_NODE);

    // Check ponger is started.
    assert!(ponger_wrapper.read().is_started());

    // Perform steps until to events.
    sim.step_until_no_events();

    // In the end, check that both processes are stopped.
    assert!(pinger_wrapper.read().is_stopped());
    assert!(ponger_wrapper.read().is_stopped());
}
