use chat::{
    client::{io::Info, requests::ClientRequestKind},
    server::process::ServerProcess,
    ClientProcess,
};
use dsbuild::{Address, Sim};

#[test]
fn stress_no_faults_2_users() {
    let mut sys = Sim::new(12345);

    sys.set_network_delays(1.0, 3.0);
    sys.set_network_drop_rate(0.05);

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
    sys.connect_node_to_network("client1_node");

    sys.add_node("client2_node", &client2_addr.host, client2_addr.port);
    sys.connect_node_to_network("client2_node");

    sys.add_node_with_storage("server_node", &server_addr.host, server_addr.port, 1 << 20);
    sys.connect_node_to_network("server_node");

    sys.add_process(
        &client1_addr.process_name,
        ClientProcess::new(
            server_addr.clone(),
            client1_addr.clone(),
            "client1".into(),
            "pass123".into(),
        ),
        "client1_node",
    );

    sys.add_process(
        &client2_addr.process_name,
        ClientProcess::new(
            server_addr.clone(),
            client2_addr.clone(),
            "client2".into(),
            "pass123".into(),
        ),
        "client2_node",
    );

    sys.add_process(
        &server_addr.process_name,
        ServerProcess::default(),
        "server_node",
    );

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
        .map(|m| m.data::<Info>().unwrap())
        .collect();

    let client2_events: Vec<_> = sys
        .step_until_local_message("client2", "client2_node")
        .unwrap()
        .into_iter()
        .map(|m| m.data::<Info>().unwrap())
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
    let client1_msg = sys.read_local_messages("client1", "client1_node").unwrap();
    let client2_msg = sys.read_local_messages("client2", "client2_node").unwrap();

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
    assert_eq!(client2_msg.unwrap().len(), 1);

    // Disconnect the second client from the chat.
    sys.send_local_message(
        "client2",
        "client2_node",
        ClientRequestKind::Disconnect.into(),
    );

    sys.step_until_no_events();

    // Check the first client did not got message.
    let client1_msg = sys.read_local_messages("client1", "client1_node");
    assert!(client1_msg.is_none());

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

    let first_history = sys.read_local_messages("client1", "client1_node").unwrap();
    assert_eq!(first_history.len(), 1 + 1 + 1 + 100 + 100 + 1 + 1 + 1 + 1);

    let second_history = sys.read_local_messages("client2", "client2_node").unwrap();
    assert_eq!(second_history.len(), 1 + 1 + 1 + 100 + 100 + 1 + 1 + 1 + 1);

    assert_eq!(first_history, second_history);
}

#[test]
fn stress_no_faults_10_users() {
    let mut sys = Sim::new(12345);

    sys.set_network_delays(1.0, 3.0);
    sys.set_network_drop_rate(0.8);

    // Add server
    let server: &'static str = "server";
    let server_addr = Address {
        host: server.into(),
        port: 1000,
        process_name: server.into(),
    };
    sys.add_node_with_storage(server, &server_addr.host, server_addr.port, 1 << 20);
    sys.connect_node_to_network(server);
    sys.add_process(&server_addr.process_name, ServerProcess::default(), server);

    // Add clients
    let clients: Vec<String> = (1..=10).map(|id| format!("client_{}", id)).collect();

    for client in clients.as_slice().iter() {
        let client_addr = Address {
            host: client.clone(),
            port: 1000,
            process_name: client.clone(),
        };
        sys.add_node(
            &client_addr.process_name,
            &client_addr.host,
            client_addr.port,
        );
        sys.connect_node_to_network(&client_addr.process_name);
        sys.add_process(
            &client_addr.process_name,
            ClientProcess::new(
                server_addr.clone(),
                client_addr.clone(),
                client_addr.process_name.clone(),
                "pass123".into(),
            ),
            &client_addr.process_name,
        );
    }

    // First client creates chat.
    let chat: &'static str = "chat";
    sys.send_local_message(
        &clients[0],
        &clients[0],
        ClientRequestKind::Create(chat.into()).into(),
    );
    sys.step_until_no_events();

    // All clients connect to created chat.
    for client in clients.as_slice().iter() {
        sys.send_local_message(
            client,
            client,
            ClientRequestKind::Connect(chat.into()).into(),
        );
    }
    sys.step_until_no_events();

    // All clients will send random messages.
    let iters = 100;
    for iter in 0..iters {
        for client in clients.as_slice().iter() {
            sys.send_local_message(
                client,
                client,
                ClientRequestKind::SendMessage(format!("msg_{}_{}", client, iter)).into(),
            );
        }

        sys.make_steps(25);
    }

    sys.step_until_no_events();

    let ref_history = sys.read_local_messages(&clients[0], &clients[0]).unwrap();
    assert!(ref_history.len() >= iters * clients.len());

    for client in clients.as_slice().iter().skip(1) {
        let history = sys.read_local_messages(client, client).unwrap();
        assert_eq!(history, ref_history);
    }
}
