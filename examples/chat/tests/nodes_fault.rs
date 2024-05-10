use chat::{
    client::{io::Info, requests::ClientRequestKind},
    server::process::ServerProcess,
    utils::sim::read_history_from_info,
    Client,
};
use dsbuild::{Address, Message, VirtualSystem};

#[test]
fn servers_fault_2_users() {
    let mut sys = VirtualSystem::new(12345);

    sys.network().set_corrupt_rate(0.0);
    sys.network().set_delays(0.5, 1.0);
    sys.network().set_drop_rate(0.05);

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

    let server1_addr = Address {
        host: "server1".into(),
        port: 10024,
        process_name: "server1".into(),
    };

    let server2_addr = Address {
        host: "server2".into(),
        port: 10024,
        process_name: "server2".into(),
    };

    sys.add_node("client1_node", &client1_addr.host, client1_addr.port);
    sys.network().connect_node("client1_node");

    sys.add_node("client2_node", &client2_addr.host, client2_addr.port);
    sys.network().connect_node("client2_node");

    sys.add_node_with_storage(
        "server1_node",
        &server1_addr.host,
        server1_addr.port,
        1 << 20,
    );
    sys.network().connect_node("server1_node");

    sys.add_node_with_storage(
        "server2_node",
        &server2_addr.host,
        server2_addr.port,
        1 << 20,
    );
    sys.network().connect_node("server2_node");

    sys.add_process(
        &client1_addr.process_name,
        Client::new_with_replica(
            server1_addr.clone(),
            server2_addr.clone(),
            client1_addr.clone(),
            "client1".into(),
            "pass123client1".into(),
        ),
        "client1_node".into(),
    );

    sys.add_process(
        &client2_addr.process_name,
        Client::new_with_replica(
            server1_addr.clone(),
            server2_addr.clone(),
            client2_addr.clone(),
            "client2".into(),
            "pass123client2".into(),
        ),
        "client2_node".into(),
    );

    sys.add_process(
        &server1_addr.process_name,
        ServerProcess::new_with_replica(server2_addr.clone()),
        "server1_node",
    );

    sys.add_process(
        &server2_addr.process_name,
        ServerProcess::new_with_replica(server1_addr.clone()),
        "server2_node",
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
    for iter in 0..15 {
        if iter % 5 == 0 {
            sys.crash_node("server1_node");
        } else if iter % 5 == 1 {
            sys.recover_node("server1_node");
            sys.network().connect_node("server1_node");
            sys.add_process(
                &server1_addr.process_name,
                ServerProcess::new_with_replica(server2_addr.clone()),
                "server1_node",
            );
            sys.send_local_message(
                &server1_addr.process_name,
                "server1_node",
                Message::new("download_events_from_replica", &String::new()).unwrap(),
            );
        } else if iter % 5 == 2 {
            sys.crash_node("server2_node");
        } else if iter % 5 == 3 {
            sys.recover_node("server2_node");
            sys.network().connect_node("server2_node");
            sys.add_process(
                &server2_addr.process_name,
                ServerProcess::new_with_replica(server1_addr.clone()),
                "server2_node",
            );
            sys.send_local_message(
                &server2_addr.process_name,
                "server2_node",
                Message::new("download_events_from_replica", &String::new()).unwrap(),
            );
        }

        sys.step_until_no_events();

        sys.send_local_message(
            "client1",
            "client1_node",
            ClientRequestKind::Connect("chat".to_owned()).into(),
        );

        sys.send_local_message(
            "client2",
            "client2_node",
            ClientRequestKind::Connect("chat".to_owned()).into(),
        );

        sys.step_until_no_events();

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

        sys.step_until_no_events();
    }

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

    let first = read_history_from_info(&mut sys, "client1_node", "client1");
    let second = read_history_from_info(&mut sys, "client2_node", "client2");
    assert_eq!(first, second);
}
