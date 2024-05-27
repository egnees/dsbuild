use chat::{
    client::{io::Info, requests::ClientRequestKind},
    utils::sim::build_sim_without_replica,
};
use dsbuild::{Address, VirtualSystem};

#[test]
fn capital_letter_chat_name() {
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
        ClientRequestKind::Create("Chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    sys.send_local_message(
        &client_addr.process_name,
        "Client",
        ClientRequestKind::Connect("chat".to_owned()).into(),
    );
    sys.step_until_no_events();

    let msg = sys
        .read_local_messages(&client_addr.process_name, "Client")
        .unwrap();
    for m in msg {
        let info = m.get_data::<Info>().unwrap();
        match info {
            Info::InnerInfo(_) => {}
            Info::ChatEvent(_) => panic!("system is not sensitive to chat name register"),
        }
    }
}
