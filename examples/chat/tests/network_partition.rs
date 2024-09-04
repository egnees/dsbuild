use chat::{
    client::requests::ClientRequestKind,
    utils::sim::{build_sim, read_history_from_info},
};
use dsbuild::{Address, Sim};

#[test]
#[should_panic]
fn network_partition_works() {
    let mut sys = Sim::new(12345);

    let client1 = Address {
        host: "Client1".into(),
        port: 10024,
        process_name: "Client1".into(),
    };

    let client2 = Address {
        host: "Client2".into(),
        port: 10024,
        process_name: "Client2".into(),
    };

    let primary = Address {
        host: "Primary".into(),
        port: 10024,
        process_name: "Primary".into(),
    };

    let replica = Address {
        host: "Replica".into(),
        port: 10024,
        process_name: "Replica".into(),
    };

    build_sim(
        &mut sys,
        vec![client1.clone(), client2.clone()].as_slice(),
        primary.clone(),
        replica.clone(),
    );

    sys.send_local_message(
        &client1.process_name,
        "Client1",
        ClientRequestKind::Create("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    sys.split_network(&["Client1", "Primary"], &["Client2", "Replica"]);
    sys.step_until_no_events();

    sys.send_local_message(
        &client1.process_name,
        "Client1",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.send_local_message(
        &client2.process_name,
        "Client2",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        &client1.process_name,
        "Client1",
        ClientRequestKind::Disconnect.into(),
    );
    sys.send_local_message(
        &client2.process_name,
        "Client2",
        ClientRequestKind::Disconnect.into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        &client1.process_name,
        "Client1",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.send_local_message(
        &client2.process_name,
        "Client2",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    let chat_history_from_first_client =
        read_history_from_info(&mut sys, "Client1", &client1.process_name);

    let chat_history_from_second_client =
        read_history_from_info(&mut sys, "Client2", &client2.process_name);

    assert_eq!(chat_history_from_first_client.len(), 4);
    assert_eq!(chat_history_from_second_client.len(), 4);

    assert_eq!(
        chat_history_from_first_client,
        chat_history_from_second_client
    );
}
