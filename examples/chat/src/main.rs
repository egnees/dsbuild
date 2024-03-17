use client::Client;
use dsbuild::{Message, RealSystem};
use log::info;

mod client;

fn main() {
    // Init logging.
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut system = RealSystem::new(1024, "127.0.0.1", 10024);

    let client = Client::default();

    let wrapper = system.add_process(client, "client".to_owned());

    let sender = wrapper.sender;
    let mut receiver = wrapper.receiver;

    system.spawn(async move {
        sender
            .send(Message::borrow_new("MSG", "Hello".to_string()).unwrap())
            .await
            .unwrap();
    });

    system.spawn(async move {
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.get_tip(), "MSG");

        info!("Got message from client: {:?}", msg);
    });

    system.run();
}
