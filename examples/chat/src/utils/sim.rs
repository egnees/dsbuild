use dsbuild::{Address, VirtualSystem};

use crate::{
    client::io::Info,
    server::{event::ChatEvent, messages::ServerMessage, process::ServerProcess},
    ClientProcess,
};

pub fn read_history(sys: &mut VirtualSystem, node: &str, proc: &str) -> Vec<ChatEvent> {
    let mut events = sys
        .read_local_messages(proc, node)
        .unwrap()
        .into_iter()
        .map(|msg| msg.get_data::<ServerMessage>().unwrap())
        .filter(|msg| match msg {
            ServerMessage::RequestResponse(_, _) => false,
            ServerMessage::ChatEvent(_, _) => true,
        })
        .map(|msg| match msg {
            ServerMessage::RequestResponse(_, _) => panic!("impossible"),
            ServerMessage::ChatEvent(_, event) => event,
        })
        .collect::<Vec<ChatEvent>>();
    events.sort();
    events.dedup();
    events
}

pub fn read_history_from_info(sys: &mut VirtualSystem, node: &str, proc: &str) -> Vec<ChatEvent> {
    let mut events = sys
        .read_local_messages(proc, node)
        .unwrap()
        .into_iter()
        .map(|msg| msg.get_data::<Info>().unwrap())
        .filter(|msg| match msg {
            Info::InnerInfo(_) => false,
            Info::ChatEvent(_) => true,
        })
        .map(|msg| match msg {
            Info::InnerInfo(_) => panic!("impossible"),
            Info::ChatEvent(event) => event,
        })
        .collect::<Vec<ChatEvent>>();
    events.sort();
    events.dedup();
    events
}

pub fn default_pass() -> String {
    "pass123".to_owned()
}

pub fn build_sim(sys: &mut VirtualSystem, clients: &[Address], server: Address, replica: Address) {
    sys.network().set_corrupt_rate(0.0);
    sys.network().set_delays(0.5, 1.0);
    sys.network().set_drop_rate(0.05);

    for client in clients {
        sys.add_node(&client.process_name, &client.host, client.port);
        sys.network().connect_node(&client.process_name);
        sys.add_process(
            &client.process_name,
            ClientProcess::new_with_replica(
                server.clone(),
                replica.clone(),
                client.clone(),
                client.process_name.clone(),
                default_pass(),
            ),
            &client.process_name,
        );
    }

    sys.add_node_with_storage(&server.process_name, &server.host, server.port, 1 << 20);
    sys.network().connect_node(&server.process_name);
    sys.add_process(
        &server.process_name,
        ServerProcess::new_with_replica(replica.clone()),
        &server.process_name,
    );

    sys.add_node_with_storage(&replica.process_name, &replica.host, replica.port, 1 << 20);
    sys.network().connect_node(&replica.process_name);
    sys.add_process(
        &replica.process_name,
        ServerProcess::new_with_replica(server.clone()),
        &replica.process_name,
    );
}

pub fn build_sim_without_replica(sys: &mut VirtualSystem, clients: &[Address], server: Address) {
    sys.network().set_corrupt_rate(0.0);
    sys.network().set_delays(0.5, 1.0);
    sys.network().set_drop_rate(0.05);

    for client in clients {
        sys.add_node(&client.process_name, &client.host, client.port);
        sys.network().connect_node(&client.process_name);
        sys.add_process(
            &client.process_name,
            ClientProcess::new(
                server.clone(),
                client.clone(),
                client.process_name.clone(),
                default_pass(),
            ),
            &client.process_name,
        );
    }

    sys.add_node_with_storage(&server.process_name, &server.host, server.port, 1 << 20);
    sys.network().connect_node(&server.process_name);
    sys.add_process(
        &server.process_name,
        ServerProcess::default(),
        &server.process_name,
    );
}

pub fn stop_server(sys: &mut VirtualSystem, server_node: &str, with_crash: bool) {
    if with_crash {
        sys.crash_node(server_node);
    } else {
        sys.shutdown_node(server_node);
    }
}

pub fn rerun_server(
    sys: &mut VirtualSystem,
    server_node: &str,
    server_addr: &Address,
    replica_addr: &Address,
    with_recovery: bool,
) {
    if with_recovery {
        sys.recover_node(server_node);
    } else {
        sys.rerun_node(server_node);
    }
    sys.network().connect_node(server_node);
    sys.add_process(
        &server_addr.process_name,
        ServerProcess::new_with_replica(replica_addr.clone()),
        server_node,
    );
}
