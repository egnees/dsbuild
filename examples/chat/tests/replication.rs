use chat::{
    client::requests::ClientRequestKind,
    utils::{
        server::check_replica_request,
        sim::{build_sim, read_history_from_info, rerun_server, stop_server},
    },
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn replication_works() {
    let mut sys = VirtualSystem::new(12345);

    let primary_addr = Address::new_ref("primary", 0, "Primary");
    let replica_addr = Address::new_ref("replica", 0, "Replica");

    build_sim(
        &mut sys,
        vec![
            Address::new_ref("client1", 0, "Client1"),
            Address::new_ref("client2", 0, "Client2"),
        ]
        .as_slice(),
        primary_addr.clone(),
        replica_addr.clone(),
    );
    sys.send_local_message("Primary", "Primary", check_replica_request());
    sys.send_local_message("Replica", "Replica", check_replica_request());
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::Create("Chat".to_string()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::Connect("Chat".to_string()).into(),
    );
    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::Connect("Chat".to_string()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message("Client1", "Client1", ClientRequestKind::Status.into());
    sys.send_local_message("Client2", "Client2", ClientRequestKind::Status.into());
    sys.step_until_no_events();

    let client1_history = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_history = read_history_from_info(&mut sys, "Client2", "Client2");

    assert_eq!(client1_history.len(), 3);
    assert_eq!(client1_history, client2_history);

    stop_server(&mut sys, "Primary", true);
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::SendMessage("Client1 message after primary fault".to_string()).into(),
    );
    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::SendMessage("Client2 message after primary fault".to_string()).into(),
    );
    sys.step_until_no_events();

    let client1_new_messages = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_new_messages = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client1_new_messages.len(), 2);
    assert_eq!(client1_new_messages, client2_new_messages);

    rerun_server(&mut sys, "Primary", &primary_addr, &replica_addr, true);
    sys.step_until_no_events();
    sys.send_local_message(
        &primary_addr.process_name,
        "Primary",
        check_replica_request(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::SendMessage("Client1 message after primary rerun".to_string()).into(),
    );
    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::SendMessage("Client2 message after primary rerun".to_string()).into(),
    );
    sys.step_until_no_events();

    let client1_new_messages = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_new_messages = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client1_new_messages.len(), 2);
    assert_eq!(client1_new_messages, client2_new_messages);
}
