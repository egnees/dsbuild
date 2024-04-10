use crate::{
    client::{
        io::Info,
        requests::RequestBuilder,
        state::{State, StateUpdateResult},
    },
    server::{chat_event::ChatEvent, messages::ServerMessageBuilder},
};

use super::{chat::Chat, parser, requests::ClientRequestKind};

#[test]
fn state() {
    // Init state.
    // Auth required.
    let mut state = State::default();

    let mut request_builder = RequestBuilder::new("123".to_owned(), "345".to_owned());
    let server_msg_builder = ServerMessageBuilder::new("server".to_owned());

    // Try send message.
    let send_msg_request = request_builder.send_message_request("message".to_owned());
    let update_result = state.apply_client_request(send_msg_request);
    assert!(update_result.to_server.is_none());
    assert!(update_result.to_user.len() == 1);
    let update_result = &update_result.to_user[0];
    assert!(matches!(update_result, Info::InnerInfo(..)));

    // Make auth request.
    let auth_request = request_builder.auth_request();
    let auth_request_id = auth_request.id;
    let update_result = state.apply_client_request(auth_request.clone());
    assert_eq!(update_result.to_server, Some(auth_request));
    assert!(update_result.to_user.is_empty());

    // Send message in chat, nothing must happen because auth is not completed yet.
    let chat_message = server_msg_builder.new_chat_event(ChatEvent::message_sent(
        "chat1".to_owned(),
        "Ivan".to_owned(),
        "Hello".to_owned(),
        0,
    ));
    let update_result = state.apply_server_msg(chat_message);
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.is_empty());

    // Send auth response, "auth done" message should be sent to user.
    let auth_response = server_msg_builder.new_good_response(auth_request_id);
    let update_result = state.apply_server_msg(auth_response);
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.len() == 1);
    let update_result = &update_result.to_user[0];
    assert!(matches!(update_result, Info::InnerInfo(..)));

    // Send connect to chat request, no new info for user should be.
    let connect_chat_request = request_builder.connect_request("chat1".to_owned());
    let connect_chat_request_id = connect_chat_request.id;
    let update_result = state.apply_client_request(connect_chat_request.clone());
    assert_eq!(update_result.to_server, Some(connect_chat_request));
    assert!(update_result.to_user.is_empty());

    // Send connect to chat response, no messages should appear to user, because message in this chat was before auth were done.
    let connect_chat_response = server_msg_builder.new_good_response(connect_chat_request_id);
    let update_result = state.apply_server_msg(connect_chat_response);
    assert_eq!(update_result, StateUpdateResult::from_nothing());

    // Now server will send all messages in the connected chat.
    let chat_message = server_msg_builder.new_chat_event(ChatEvent::message_sent(
        "chat1".to_owned(),
        "Ivan".to_owned(),
        "Hello".to_owned(),
        0,
    ));
    let update_result = state.apply_server_msg(chat_message);
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.len() == 1);

    // Messages from the other chat must be ignored.
    let chat_message = server_msg_builder.new_chat_event(ChatEvent::message_sent(
        "chat2".to_owned(),
        "Ivan".to_owned(),
        "Hello".to_owned(),
        0,
    ));
    let update_result = state.apply_server_msg(chat_message);
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.is_empty());

    // User requests disconnect.
    let disconnect_request = request_builder.disconnect_request();
    let disconnect_request_id = disconnect_request.id;
    let update_result = state.apply_client_request(disconnect_request.clone());
    assert_eq!(update_result.to_server, Some(disconnect_request));
    assert!(update_result.to_user.is_empty());

    // Now appeared messages must be ignored.
    let chat_message = server_msg_builder.new_chat_event(ChatEvent::message_sent(
        "chat1".to_owned(),
        "Ivan".to_owned(),
        "Hello".to_owned(),
        1,
    ));
    let update_result = state.apply_server_msg(chat_message);
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.is_empty());

    // Now disconnect request is ignored by server,
    // which leads to pending messages appear,
    // but disconnect response must be before this messages.
    let disconnect_response =
        server_msg_builder.new_bad_response(disconnect_request_id, "disconnect ignored".into());
    let update_result = state.apply_server_msg(disconnect_response);
    assert_eq!(update_result.to_server, None);
    assert_eq!(update_result.to_user.len(), 2);
    assert!(matches!(update_result.to_user[0], Info::InnerInfo(..)));
    assert!(matches!(update_result.to_user[1], Info::ChatEvent(..)));

    // Now client send two messages to the chat.
    // One of them must be pending while the response on the first one is returned.
    let send_msg_request = request_builder.send_message_request("message".to_owned());
    let send_msg_request_id = send_msg_request.id;
    let update_result = state.apply_client_request(send_msg_request.clone());
    assert_eq!(update_result.to_server, Some(send_msg_request));
    assert!(update_result.to_user.is_empty());

    let send_msg_request_2 = request_builder.send_message_request("message2".to_owned());
    let send_msg_request_2_id = send_msg_request_2.id;
    let update_result = state.apply_client_request(send_msg_request_2.clone());
    assert_eq!(update_result.to_server, None);
    assert!(update_result.to_user.is_empty());

    // Now server will respond on the first message.
    // Check that the pending message will be returned to client,
    // so client can send it.
    let send_msg_response = server_msg_builder.new_good_response(send_msg_request_id);
    let update_result = state.apply_server_msg(send_msg_response);
    assert_eq!(update_result.to_server, Some(send_msg_request_2.clone()));
    assert!(update_result.to_user.is_empty());

    // Make it for the second message also.
    let send_msg_response = server_msg_builder.new_good_response(send_msg_request_2_id);
    let update_result = state.apply_server_msg(send_msg_response);
    assert_eq!(update_result, StateUpdateResult::from_nothing());
}

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
        assert_eq!(event.seq, i);
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
