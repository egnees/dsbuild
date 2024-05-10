use dsbuild::VirtualSystem;

use crate::{
    client::io::Info,
    server::{event::ChatEvent, messages::ServerMessage},
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
