use dsbuild::{Address, RealNode};
use dsbuild_message::Tipped;

use crate::process::{
    LocalPingRequest, Ping, PingPongProcess, Pong,
};

/// Allows to run ponger process.
pub fn ponger() {
    // Create current node runtime and add pinger process on it.
    let mut node =
        RealNode::new("127.0.0.1", 10095, ".system/");
    let mut proc = node.add_process(
        PingPongProcess::default(),
        "ponger".into(),
    );

    // Schedule asyncronous activity.
    node.spawn(async move {
        // Wait for ping.
        let message = proc.receiver.recv().await.unwrap();
        assert_eq!(message.get_tip(), Ping::TIP);

        // Stop process after ping
        // received and pong sended.
        proc.stop().await;
    });

    // Run scheduled activities and processes.
    node.run();
}

/// Allows to run pinger process.
pub fn pinger() {
    // Set ponger address
    let ponger =
        Address::new_ref("127.0.0.1", 10095, "ponger");

    // Create current node runtime and add pinger process on it.
    let mut node =
        RealNode::new("127.0.0.1", 10094, ".system/");
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

        // Stop process after pong received.
        proc.stop().await;
    });

    // Run scheduled activities and processes.
    node.run();
}
