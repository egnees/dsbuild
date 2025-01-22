use dsbuild::Sim;
use dsbuild_message::Tipped;
use pingpong::process::{
    LocalPingRequest, PingPongProcess, Pong,
};

#[test]
fn basic() {
    // Instantiate simulation from random seed.
    let mut sim = Sim::new(123);

    // Configure network in delay and drop rate.
    sim.set_network_delays(0.1, 0.2);
    sim.set_network_drop_rate(0.0);

    // Add two nodes in the system.
    sim.add_node_with_storage(
        "node1",
        "10.12.0.1",
        10024,
        1 << 20,
    );
    sim.add_node_with_storage(
        "node2",
        "10.12.0.2",
        10024,
        1 << 20,
    );

    // Add processes on nodes.
    let p1 = sim.add_process(
        "pinger",
        PingPongProcess::default(),
        "node1",
    );
    let p2 = sim.add_process(
        "ponger",
        PingPongProcess::default(),
        "node2",
    );

    // Send local ping request to pinger.
    sim.send_local_message(
        "pinger",
        "node1",
        LocalPingRequest {
            receiver: p2.address.clone(),
        }
        .into(),
    );

    // Read pinger local messages pinger process and expect for pong.
    let msgs = sim
        .step_until_local_message("pinger", "node1")
        .unwrap();
    assert_eq!(msgs[0].get_tip(), Pong::TIP);

    // Expect correct ping and pong counts.
    assert_eq!(p1.read().pongs_received, 1);
    assert_eq!(p2.read().pings_received, 1);
}
