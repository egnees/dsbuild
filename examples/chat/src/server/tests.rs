use dsbuild::Address;

use crate::{
    client::requests::RequestBuilder,
    server::{messages::ServerMessageKind, state::State},
};

#[test]
fn test_state() {
    let mut state = State::new("server123".to_owned());

    let mut user1_request_builder = RequestBuilder::new("user1".to_owned(), "pass123".to_owned());

    let mut user2_request_builder = RequestBuilder::new("user2".to_owned(), "pass321".to_owned());

    let user1 = Address {
        host: "123".into(),
        port: 123,
        process_name: "user1".into(),
    };

    let user2 = Address {
        host: "345".into(),
        port: 345,
        process_name: "user2".into(),
    };

    // Request to send message, or connect to chat, or disconnect from chat must lead to error.
    let send_msg_request = user1_request_builder.send_message_request("hello".into());
    let result = state.process_client_request(user1.clone(), send_msg_request);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].to, user1);
    assert!(matches!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(..)
    ));

    // Send auth request from user1.
    let auth_request = user1_request_builder.auth_request();
    let result = state.process_client_request(user1.clone(), auth_request.clone());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].to, user1);
    assert_eq!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(auth_request.id, Ok(()))
    );

    // Send auth request from user2.
    let auth_request = user2_request_builder.auth_request();
    let result = state.process_client_request(user2.clone(), auth_request.clone());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].to, user2);
    assert_eq!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(auth_request.id, Ok(()))
    );

    // User1 creates chat.
    let create_chat_request = user1_request_builder.create_request("chat1".into());
    let result = state.process_client_request(user1.clone(), create_chat_request.clone());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].to, user1);
    assert_eq!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(create_chat_request.id, Ok(()))
    );

    // User2 tries to create the same chat.
    let create_chat_request = user2_request_builder.create_request("chat1".into());
    let result = state.process_client_request(user2.clone(), create_chat_request.clone());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].to, user2);
    assert!(matches!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(..)
    ));

    // User1 tries to connect to the chat.
    let connect_chat_request = user1_request_builder.connect_request("chat1".into());
    let result = state.process_client_request(user1.clone(), connect_chat_request.clone());
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].to, user1);
    assert!(matches!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(..)
    ));
    assert_eq!(result[1].to, user1);
    assert!(matches!(
        result[1].msg.kind,
        ServerMessageKind::ChatEvents(..)
    ));
    assert_eq!(result[2].to, user1);
    assert!(matches!(
        result[2].msg.kind,
        ServerMessageKind::ChatEvents(..)
    ));

    // User2 tries to connect to the chat.
    let connect_chat_request = user2_request_builder.connect_request("chat1".into());
    let result = state.process_client_request(user2.clone(), connect_chat_request.clone());

    assert_eq!(result.len(), 5);
    assert_eq!(result[0].to, user2);
    assert_eq!(
        result[0].msg.kind,
        ServerMessageKind::RequestResponse(connect_chat_request.id, Ok(()))
    );
}
