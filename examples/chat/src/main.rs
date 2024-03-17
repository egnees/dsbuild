use client::Client;
use dsbuild::{Address, Message, RealSystem};
use log::info;

mod client;

fn main() {
    // Init logging.
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut system = RealSystem::new(1024, "127.0.0.1", 10024);

    let client1 = Client {
        other: Address {
            host: "127.0.0.1".to_owned(),
            port: 10024,
            process_name: "client2".to_owned(),
        },
    };

    let client2 = Client {
        other: Address {
            host: "127.0.0.1".to_owned(),
            port: 10024,
            process_name: "client1".to_owned(),
        },
    };

    let wrapper = system.add_process(client1, "client1".to_owned());

    let sender1 = wrapper.sender;
    let mut receiver1 = wrapper.receiver;

    system.spawn(async move {
        sender1
            .send(Message::borrow_new("MSG", "Hello from client1".to_string()).unwrap())
            .await
            .unwrap();
    });

    system.spawn(async move {
        let msg = receiver1.recv().await.unwrap();
        assert_eq!(msg.get_tip(), "MSG");

        info!(
            "Got message from client1: {:?}",
            msg.get_data::<String>().unwrap()
        );
    });

    let wrapper = system.add_process(client2, "client2".to_owned());

    let sender2 = wrapper.sender;
    let mut receiver2 = wrapper.receiver;

    system.spawn(async move {
        sender2
            .send(Message::borrow_new("MSG", "Hello from client2".to_string()).unwrap())
            .await
            .unwrap();
    });

    system.spawn(async move {
        let msg = receiver2.recv().await.unwrap();
        assert_eq!(msg.get_tip(), "MSG");

        info!(
            "Got message from client2: {:?}",
            msg.get_data::<String>().unwrap()
        );
    });

    system.run();
}
