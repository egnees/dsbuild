use chat::{
    client::requests::ClientRequestKind, server::process::ServerProcess,
    utils::sim::read_history_from_info, Client,
};
use dsbuild::{Address, VirtualSystem};

#[test]
#[should_panic]
fn network_split_works() {
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

    sys.send_local_message(
        &client1_addr.process_name,
        "client1_node",
        ClientRequestKind::Create("chat1".to_owned()).into(),
    );

    sys.step_until_no_events();

    sys.network().make_partition(
        &["client1_node", "server1_node"],
        &["client2_node", "server2_node"],
    );

    sys.step_until_no_events();

    sys.send_local_message(
        &client1_addr.process_name,
        "client1_node",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );

    sys.send_local_message(
        &client2_addr.process_name,
        "client2_node",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );

    sys.step_until_no_events();

    sys.send_local_message(
        &client1_addr.process_name,
        "client1_node",
        ClientRequestKind::Disconnect.into(),
    );

    sys.send_local_message(
        &client2_addr.process_name,
        "client2_node",
        ClientRequestKind::Disconnect.into(),
    );

    sys.step_until_no_events();

    sys.send_local_message(
        &client1_addr.process_name,
        "client1_node",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );

    sys.send_local_message(
        &client2_addr.process_name,
        "client2_node",
        ClientRequestKind::Connect("chat1".to_owned()).into(),
    );

    sys.step_until_no_events();

    let chat_history_from_first_client =
        read_history_from_info(&mut sys, "client1_node", &client1_addr.process_name);

    let chat_history_from_second_client =
        read_history_from_info(&mut sys, "client2_node", &client2_addr.process_name);

    assert_eq!(chat_history_from_first_client.len(), 4);
    assert_eq!(chat_history_from_second_client.len(), 4);

    assert_eq!(
        chat_history_from_first_client,
        chat_history_from_second_client
    );
}
