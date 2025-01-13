use dsbuild::{Address, RealNode};
use dsbuild_message::Tipped;
use pingpong::process::{
    LocalPingRequest, PingPongProcess, Pong,
};

fn main() {
    // Set ponger address
    let ponger =
        Address::new_ref("localhost", 10095, "ponger");

    // Create current node runtime and add pinger process on it.
    let mut node =
        RealNode::new("localhost", 10094, ".system/");
    let mut proc = node.add_process(
        PingPongProcess::default(),
        "pinger".into(),
    );

    // Schedule asyncronous activity.
    node.spawn(async move {
        // Send local ping request and wait for pong response.
        let ping_request =
            LocalPingRequest { receiver: ponger };
        proc.sender
            .send(ping_request.into())
            .await
            .unwrap();
        let response = proc.receiver.recv().await.unwrap();
        assert_eq!(response.get_tip(), Pong::TIP);
    });

    // Schedule scheduled activities and processes.
    node.run();
}
