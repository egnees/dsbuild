use dsbuild::Sim;
use pingpong::process::{LocalPingRequest, PingPongProcess};

#[test]
fn test() {
    // Instantiate simulation from random seed.
    let mut sim = Sim::new(123);

    // Configure network in delay and drop rate.
    sim.set_network_delays(0.1, 0.2);
    sim.set_network_drop_rate(0.0);

    // Add two nodes in the system.
    sim.add_node_with_storage("node1", "10.12.0.1", 10024, 1 << 20);
    sim.add_node_with_storage("node2", "10.12.0.2", 10024, 1 << 20);

    // Add processes on nodes.
    let pinger = sim.add_process("pinger", PingPongProcess::default(), "node1");
    let ponger = sim.add_process("ponger", PingPongProcess::default(), "node2");
}
