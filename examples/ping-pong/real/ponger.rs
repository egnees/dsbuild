use dsbuild::RealNode;
use dsbuild_message::Tipped;
use pingpong::process::{Ping, PingPongProcess};

fn main() {
    // Create current node runtime and add pinger process on it.
    let mut node = RealNode::new("localhost", 10095, ".system/");
    let mut proc = node.add_process(PingPongProcess::default(), "ponger".into());

    // Schedule asyncronous activity.
    node.spawn(async move {
        // Wait for ping.
        let message = proc.receiver.recv().await.unwrap();
        assert_eq!(message.get_tip(), Ping::TIP);
    });

    // Schedule scheduled activities and processes.
    node.run();
}
