use chat::{
    client::{io::Info, requests::ClientRequestKind},
    Client, Server,
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn works_no_faults() {
    let mut sys = VirtualSystem::new(12345);

    sys.network().set_corrupt_rate(0.0);
    sys.network().set_delays(1.0, 3.0);
    sys.network().set_drop_rate(0.5);

    let client1_addr = Address {
        host: "client1_host".into(),
        port: 10024,
        process_name: "client1".into(),
    };

    let client2_addr = Address {
        host: "client2_host".into(),
        port: 10024,
        process_name: "client2".into(),
    };

    let server_addr = Address {
        host: "server".into(),
        port: 10024,
        process_name: "server".into(),
    };

    sys.add_node("client1_node", &client1_addr.host, client1_addr.port);
    sys.network().connect_node("client1_node");

    sys.add_node("client2_node", &client2_addr.host, client2_addr.port);
    sys.network().connect_node("client2_node");

    sys.add_node("server_node", &server_addr.host, server_addr.port);
    sys.network().connect_node("server_node");

    sys.add_process(
        &client1_addr.process_name,
        Client::new(
            server_addr.clone(),
            client1_addr.clone(),
            "client1".into(),
            "pass123".into(),
        ),
        "client1_node".into(),
    );

    sys.add_process(
        &client2_addr.process_name,
        Client::new(
            server_addr.clone(),
            client2_addr.clone(),
            "client2".into(),
            "pass123".into(),
        ),
        "client2_node".into(),
    );

    sys.add_process(
        &server_addr.process_name,
        Server::new("server".into()),
        "server_node",
    );

    sys.send_local_message("client1", "client1_node", ClientRequestKind::Auth.into());

    // Send auth request, get auth response.
    let msg = sys
        .step_until_local_message("client1", "client1_node")
        .unwrap();
    assert_eq!(msg.len(), 1);

    // Auth client2.
    sys.send_local_message("client2", "client2_node", ClientRequestKind::Auth.into());
    let msg = sys
        .step_until_local_message("client2", "client2_node")
        .unwrap();
    assert_eq!(msg.len(), 1);

    // Client1 creates chat.
    sys.send_local_message(
        "client1",
        "client1_node",
        ClientRequestKind::Create("chat".into()).into(),
    );

    // Step until chat will be created.
    sys.step_until_no_events();

    // Client2 connects to chat.
    sys.send_local_message(
        "client2",
        "client2_node",
        ClientRequestKind::Connect("chat".into()).into(),
    );

    // Client1 connects to chat too in the same time.
    sys.send_local_message(
        "client1",
        "client1_node",
        ClientRequestKind::Connect("chat".into()).into(),
    );

    // Step until messages will be delivered.
    sys.step_until_no_events();

    // The same chat events must be sent to the clients.
    let client1_events: Vec<_> = sys
        .step_until_local_message("client1", "client1_node")
        .unwrap()
        .into_iter()
        .map(|m| m.get_data::<Info>().unwrap())
        .collect();

    let client2_events: Vec<_> = sys
        .step_until_local_message("client2", "client2_node")
        .unwrap()
        .into_iter()
        .map(|m| m.get_data::<Info>().unwrap())
        .collect();

    // Check messages arrived in the same order.
    assert_eq!(client1_events, client2_events);

    // Both clients send messages in the chat.
    for iter in 0..10 {
        for i in 0..10 {
            sys.send_local_message(
                "client1",
                "client1_node",
                ClientRequestKind::SendMessage(format!("client1_{}", iter * 10 + i)).into(),
            );

            sys.send_local_message(
                "client2",
                "client2_node",
                ClientRequestKind::SendMessage(format!("client2_{}", iter * 10 + i)).into(),
            );
        }

        sys.make_steps(15);
    }

    sys.step_until_no_events();

    // Check both clients got the same messages.
    let client1_msg = sys.read_local_messages("client1", "client1_node");
    let client2_msg = sys.read_local_messages("client2", "client2_node");

    assert_eq!(client1_msg, client2_msg);

    // Disconnect the first client from the chat.
    sys.send_local_message(
        "client1",
        "client1_node",
        ClientRequestKind::Disconnect.into(),
    );

    sys.step_until_no_events();

    // Check the second client got message.
    let client2_msg = sys.read_local_messages("client2", "client2_node");
    assert_eq!(client2_msg.len(), 1);

    // Disconnect the second client from the chat.
    sys.send_local_message(
        "client2",
        "client2_node",
        ClientRequestKind::Disconnect.into(),
    );

    sys.step_until_no_events();

    // Check the first client did not got message.
    let client1_msg = sys.read_local_messages("client1", "client1_node");
    assert!(client1_msg.is_empty());

    // Connect the first client to the server.
    sys.send_local_message(
        "client1",
        "client1_node",
        ClientRequestKind::Connect("chat".into()).into(),
    );

    // Connect the second client to the server.
    sys.send_local_message(
        "client2",
        "client2_node",
        ClientRequestKind::Connect("chat".into()).into(),
    );

    sys.step_until_no_events();

    let first_history = sys.read_local_messages("client1", "client1_node");
    assert_eq!(first_history.len(), 1 + 1 + 1 + 100 + 100 + 1 + 1 + 1 + 1);

    let second_history = sys.read_local_messages("client2", "client2_node");
    assert_eq!(second_history.len(), 1 + 1 + 1 + 100 + 100 + 1 + 1 + 1 + 1);

    assert_eq!(first_history, second_history);
}