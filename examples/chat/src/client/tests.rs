use crate::server::event::ChatEvent;

use super::{chat::Chat, parser, requests::ClientRequestKind};

#[test]
fn parser() {
    assert!(matches!(parser::parse_request("request"), Err(..)));

    assert!(matches!(parser::parse_request("/send"), Err(..)));

    assert_eq!(
        parser::parse_request("/send msg"),
        Ok(ClientRequestKind::SendMessage("msg".into()))
    );

    assert_eq!(
        parser::parse_request("/send 'msg1 msg2'"),
        Ok(ClientRequestKind::SendMessage("msg1 msg2".into()))
    );

    assert!(parser::parse_request("/send one two three  four").is_err());

    assert_eq!(
        parser::parse_request("/connect chat1"),
        Ok(ClientRequestKind::Connect("chat1".into()))
    );

    assert!(parser::parse_request("/connect chat 1").is_err());

    assert_eq!(
        parser::parse_request("/disconnect"),
        Ok(ClientRequestKind::Disconnect)
    );

    assert_eq!(
        parser::parse_request("/create chat123"),
        Ok(ClientRequestKind::Create("chat123".into()))
    );

    assert!(parser::parse_request("/create chat 123").is_err());

    assert!(parser::parse_request("/disconnect 1").is_err());

    assert_eq!(
        parser::parse_request("/sEnD msg"),
        Ok(ClientRequestKind::SendMessage("msg".into()))
    );
}

#[test]
fn chat_events() {
    let event1 = ChatEvent::chat_created("123".into(), "chat".into(), 4);
    let event2 = ChatEvent::chat_created("123".into(), "chat".into(), 0);
    let event3 = ChatEvent::chat_created("123".into(), "chat".into(), 2);
    let event4 = ChatEvent::chat_created("123".into(), "chat".into(), 3);
    let event5 = ChatEvent::chat_created("123".into(), "chat".into(), 1);
    let event6 = ChatEvent::chat_created("123".into(), "chat".into(), 5);

    let mut v = vec![event1, event2, event3, event4, event5, event6];
    v.sort();

    for (i, event) in v.into_iter().enumerate() {
        assert_eq!(event.seq, i as u64);
    }
}

#[test]
fn chat() {
    let mut chat = Chat::new("chat".into());

    let created_event = ChatEvent::chat_created("123".into(), "chat".into(), 0);
    let events = chat.process_event(created_event.clone());

    assert_eq!(events.len(), 1);
    assert_eq!(events[0], created_event);

    let events = chat.process_event(created_event.clone());
    assert!(events.is_empty());

    let send_msg_2 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_2".into(), 2);
    let events = chat.process_event(send_msg_2.clone());
    assert!(events.is_empty());

    let send_msg_5 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_5".into(), 5);
    let events = chat.process_event(send_msg_5.clone());
    assert!(events.is_empty());

    let send_msg_1 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_1".into(), 1);
    let events = chat.process_event(send_msg_1.clone());
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], send_msg_1);
    assert_eq!(events[1], send_msg_2);

    let send_msg_3 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_3".into(), 3);
    let send_msg_4 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_4".into(), 4);
    let send_msg_6 = ChatEvent::message_sent("chat".into(), "123".into(), "send_msg_6".into(), 6);

    let events = chat.process_events(vec![
        send_msg_2.clone(),
        send_msg_3.clone(),
        send_msg_6.clone(),
        send_msg_4.clone(),
    ]);

    assert_eq!(events.len(), 4);
    assert_eq!(events, vec![send_msg_3, send_msg_4, send_msg_5, send_msg_6]);
}
