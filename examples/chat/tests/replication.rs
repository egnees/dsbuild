use chat::{
    client::requests::ClientRequestKind,
    utils::sim::{build_sim, read_history_from_info, rerun_server, stop_server},
};
use dsbuild::{Address, Sim};

#[test]
fn replication_works() {
    let mut sys = Sim::new(12345);

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
    sys.step_until_no_events();

    // create Chat and connect, primary=ok, replica=ok

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
    let client1_history = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_history = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client1_history.len(), 3);
    assert_eq!(client1_history, client2_history);

    // crash primary

    stop_server(&mut sys, "Primary", true);
    sys.step_until_no_events();

    // clients send messages to chat, primary=crash, replica=ok

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::SendMessage("Client1 message after primary crashed".to_string()).into(),
    );
    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::SendMessage("Client2 message after primary crashed".to_string()).into(),
    );
    sys.step_until_no_events();

    let client1_new_messages = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_new_messages = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client1_new_messages.len(), 2);
    assert_eq!(client1_new_messages, client2_new_messages);

    // recover primary

    rerun_server(&mut sys, "Primary", &primary_addr, &replica_addr, true);
    sys.step_until_no_events();

    // clients send messages, primary=ok, replica=ok

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::SendMessage("Client1 message after primary recovery".to_string()).into(),
    );
    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::SendMessage("Client2 message after primary recovery".to_string()).into(),
    );
    sys.step_until_no_events();

    let client1_new_messages = read_history_from_info(&mut sys, "Client1", "Client1");
    let client2_new_messages = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client1_new_messages.len(), 2);
    assert_eq!(client1_new_messages, client2_new_messages);

    // stop replica

    stop_server(&mut sys, "Replica", false);
    sys.step_until_no_events();

    // disconnect client1 from Chat, primary=ok, replica=stop

    sys.send_local_message("Client1", "Client1", ClientRequestKind::Disconnect.into());
    sys.step_until_no_events();

    // client1 creates NewChat, then connects, then sends message, primary=ok, replica=stop

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::Create("NewChat".to_string()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::Connect("NewChat".to_string()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        "Client1",
        "Client1",
        ClientRequestKind::SendMessage("Client1 message after replica stopped".to_string()).into(),
    );
    sys.step_until_no_events();

    // client2 disconnect from Chat, primary=ok, replica=stop

    sys.send_local_message("Client2", "Client2", ClientRequestKind::Disconnect.into());
    sys.step_until_no_events();

    // rerun replica

    rerun_server(&mut sys, "Replica", &replica_addr, &primary_addr, false);
    sys.step_until_no_events();

    // crash primary

    stop_server(&mut sys, "Primary", true);
    sys.step_until_no_events();

    // client2 connects to NewChat, primary=crash, replica=ok

    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::Connect("NewChat".to_string()).into(),
    );
    sys.step_until_no_events();

    let client2_history = read_history_from_info(&mut sys, "Client2", "Client2");
    assert_eq!(client2_history.len(), 5);

    // client2 sends message to NewChat, primary=crash, replica=ok

    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::SendMessage("Client2 message after primary crash 2".to_string()).into(),
    );
    sys.step_until_no_events();

    let client1_history = read_history_from_info(&mut sys, "Client1", "Client1");
    assert_eq!(client1_history.len(), 5);
}
