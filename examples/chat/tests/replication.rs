use chat::{
    client::{io::Info, requests::ClientRequestKind},
    utils::{log::enable_debug_log, server::check_replica_request, sim::build_sim},
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn just_works() {
    enable_debug_log();

    let mut sys = VirtualSystem::new(12345);
    build_sim(
        &mut sys,
        vec![
            Address::new_ref("client1", 0, "Client1"),
            Address::new_ref("client2", 0, "Client2"),
        ]
        .as_slice(),
        Address::new_ref("primary", 0, "Primary"),
        Address::new_ref("replica", 0, "Replica"),
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
    sys.step_until_no_events();
    let client1_local_messages = sys.read_local_messages("Client1", "Client1").unwrap();
    println!("client1_local_messages:");
    for msg in client1_local_messages {
        let info = msg.get_data::<Info>().unwrap();
        println!("{:?}", info);
    }

    sys.send_local_message(
        "Client2",
        "Client2",
        ClientRequestKind::Connect("Chat".to_string()).into(),
    );
    sys.step_until_no_events();

    let client2_local_messages = sys.read_local_messages("Client2", "Client2").unwrap();
    println!("client2_local_messages:");
    for msg in client2_local_messages {
        let info = msg.get_data::<Info>().unwrap();
        println!("{:?}", info);
    }
}
