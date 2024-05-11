use chat::{
    client::requests::ClientRequestKind,
    utils::sim::{build_sim_without_replica, default_pass, read_history_from_info},
    ClientProcess,
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn client_reconnect() {
    let mut sys = VirtualSystem::new(12345);

    let primary_addr = Address::new_ref("primary", 0, "Primary");

    let client_addr = Address::new_ref("client", 0, "Client");

    build_sim_without_replica(
        &mut sys,
        vec![client_addr.clone()].as_slice(),
        primary_addr.clone(),
    );

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::Create("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::SendMessage("msg".to_owned()).into(),
    );
    sys.step_until_no_events();

    assert_eq!(
        read_history_from_info(&mut sys, "Client", &client_addr.process_name).len(),
        3
    );

    sys.crash_node("Client");
    sys.step_until_no_events();

    sys.recover_node("Client");
    sys.add_process(
        &client_addr.process_name,
        ClientProcess::new(
            primary_addr.clone(),
            client_addr.clone(),
            client_addr.process_name.clone(),
            default_pass(),
        ),
        "Client",
    );
    sys.network().connect_node("Client");
    sys.step_until_no_events();

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::Status.into(),
    );
    sys.step_until_no_events();

    assert_eq!(
        read_history_from_info(&mut sys, "Client", &client_addr.process_name).len(),
        3
    );

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::SendMessage("message after client reconnect".to_string()).into(),
    );
    sys.step_until_no_events();

    assert_eq!(
        read_history_from_info(&mut sys, "Client", &client_addr.process_name).len(),
        1
    );
}
