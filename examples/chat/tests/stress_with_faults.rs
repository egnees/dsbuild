use chat::{
    client::requests::ClientRequestKind,
    utils::{
        server::check_replica_request,
        sim::{build_sim, read_history_from_info, rerun_server, stop_server},
    },
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn stress_with_faults_2_users() {
    let mut sys = VirtualSystem::new(12345);

    let client1_addr = Address {
        host: "client1".into(),
        port: 10024,
        process_name: "Client1".into(),
    };

    let client2_addr = Address {
        host: "client2".into(),
        port: 10024,
        process_name: "Client2".into(),
    };

    let primary = Address {
        host: "primary".into(),
        port: 10024,
        process_name: "Primary".into(),
    };

    let replica = Address {
        host: "replica".into(),
        port: 10024,
        process_name: "Replica".into(),
    };

    build_sim(
        &mut sys,
        vec![client1_addr.clone(), client2_addr.clone()].as_slice(),
        primary.clone(),
        replica.clone(),
    );

    // Client1 creates chat.
    sys.send_local_message(
        &client1_addr.process_name,
        "Client1",
        ClientRequestKind::Create("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    // Client2 connects to chat.
    sys.send_local_message(
        &client2_addr.process_name,
        "Client2",
        ClientRequestKind::Connect("chat".into()).into(),
    );

    // Client1 connects to chat too in the same time.
    sys.send_local_message(
        &client1_addr.process_name,
        "Client1",
        ClientRequestKind::Connect("chat".into()).into(),
    );
    sys.step_until_no_events();

    let first_client_history =
        read_history_from_info(&mut sys, "Client1", &client1_addr.process_name);
    let second_client_history =
        read_history_from_info(&mut sys, "Client2", &client2_addr.process_name);
    assert_eq!(first_client_history.len(), 3);
    assert_eq!(first_client_history, second_client_history);

    // Both clients send messages in the chat.
    for iter in 0..15 {
        if iter % 5 == 0 {
            stop_server(&mut sys, "Primary", true);
        } else if iter % 5 == 1 {
            rerun_server(&mut sys, "Primary", &primary, &replica, true);
            sys.send_local_message(&primary.process_name, "Primary", check_replica_request());
        } else if iter % 5 == 2 {
            stop_server(&mut sys, "Replica", true);
        } else if iter % 5 == 3 {
            rerun_server(&mut sys, "Replica", &replica, &primary, true);
            sys.send_local_message(&replica.process_name, "Replica", check_replica_request());
        }
        sys.step_until_no_events();

        sys.send_local_message(
            &client1_addr.process_name,
            "Client1",
            ClientRequestKind::Connect("chat".to_owned()).into(),
        );
        sys.send_local_message(
            &client2_addr.process_name,
            "Client2",
            ClientRequestKind::Connect("chat".to_owned()).into(),
        );
        sys.step_until_no_events();

        for i in 0..10 {
            sys.send_local_message(
                &client1_addr.process_name,
                "Client1",
                ClientRequestKind::SendMessage(format!("client1_{}", iter * 10 + i)).into(),
            );

            sys.send_local_message(
                &client2_addr.process_name,
                "Client2",
                ClientRequestKind::SendMessage(format!("client2_{}", iter * 10 + i)).into(),
            );
        }
        sys.step_until_no_events();

        sys.send_local_message(
            &client1_addr.process_name,
            "Client1",
            ClientRequestKind::Disconnect.into(),
        );
        sys.send_local_message(
            &client2_addr.process_name,
            "Client2",
            ClientRequestKind::Disconnect.into(),
        );
        sys.step_until_no_events();
    }

    sys.send_local_message(
        &client1_addr.process_name,
        "Client1",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.send_local_message(
        &client2_addr.process_name,
        "Client2",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    let first = read_history_from_info(&mut sys, "Client1", &client1_addr.process_name);
    let second = read_history_from_info(&mut sys, "Client2", &client2_addr.process_name);
    assert_eq!(first, second);
}
