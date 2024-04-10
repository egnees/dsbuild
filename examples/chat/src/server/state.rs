use std::collections::HashMap;

use dsbuild::Address;

use crate::client::requests::{ClientRequest, ClientRequestKind};

use super::{
    chat::Chat,
    chat_event::ChatEvent,
    messages::{ServerMessage, ServerMessageBuilder},
    user::User,
};

pub struct State {
    chats: HashMap<String, Chat>,
    auth_users: HashMap<String, User>,
    msg_builder: ServerMessageBuilder,
}

#[derive(Clone, Debug)]
pub struct RoutedMessage {
    pub to: Address,
    pub msg: ServerMessage,
}

impl State {
    pub fn new(server: String) -> Self {
        Self {
            chats: HashMap::new(),
            auth_users: HashMap::new(),
            msg_builder: ServerMessageBuilder::new(server),
        }
    }

    pub fn get_user_by_address(&self, address: Address) -> Option<String> {
        for (name, user) in self.auth_users.clone() {
            if user.address() == address {
                return Some(name);
            }
        }

        None
    }

    pub fn remove_auth_for_user(&mut self, address: Address) -> Vec<RoutedMessage> {
        let user_name = self.get_user_by_address(address.clone());
        if user_name.is_none() {
            return Vec::new();
        }

        let user_name = user_name.unwrap();
        let user = self.auth_users.get(user_name.as_str()).unwrap();
        let user_name = user.name().to_owned();
        let user_password = user.password().to_owned();

        let messages = if user.is_connected_to_chat() {
            self.disconnect_user(&address, user_name.as_str(), user_password.as_str())
                .unwrap()
        } else {
            Vec::new()
        };

        self.auth_users.remove(user_name.as_str()).unwrap();

        messages
    }

    pub fn process_client_request(
        &mut self,
        from: Address,
        request: ClientRequest,
    ) -> Vec<RoutedMessage> {
        let user = request.client.as_str();
        let password = request.password.as_str();
        let address = &from;

        let result = match &request.kind {
            ClientRequestKind::Auth => self.auth_user(address, user, password),
            ClientRequestKind::SendMessage(msg) => {
                self.user_send_message_in_chat(address, user, password, msg.clone())
            }
            ClientRequestKind::Create(chat_name) => {
                self.user_create_chat(address, user, password, chat_name.clone())
            }
            ClientRequestKind::Connect(chat_name) => {
                self.connect_user_to_chat(chat_name.as_str(), address, user, password)
            }
            ClientRequestKind::Disconnect => self.disconnect_user(address, user, password),
        };

        match result {
            Ok(mut messages) => {
                let ret = self.msg_builder.new_good_response(request.id);
                let mut ret_messages = vec![RoutedMessage { to: from, msg: ret }];
                ret_messages.append(&mut messages);
                ret_messages
            }
            Err(info) => {
                let ret = self.msg_builder.new_bad_response(request.id, info);
                let ret_messages = vec![RoutedMessage { to: from, msg: ret }];
                ret_messages
            }
        }
    }

    fn auth_user(
        &mut self,
        address: &Address,
        user: &str,
        password: &str,
    ) -> Result<Vec<RoutedMessage>, String> {
        if !self.auth_users.contains_key(user) {
            self.auth_users.insert(
                user.to_owned(),
                User::new(address.clone(), user.to_owned(), password.to_owned()),
            );

            Ok(Vec::new())
        } else {
            self.verify_user(address, user, password)
                .map(|_| Vec::new())
        }
    }

    fn verify_user(&self, address: &Address, user: &str, password: &str) -> Result<(), String> {
        if !self.auth_users.contains_key(user) {
            Err("auth error".into())
        } else {
            let verify = self
                .auth_users
                .get(user)
                .unwrap()
                .verify(address, user, password);

            if verify {
                Ok(())
            } else {
                Err("incorrect credentials or untrusted address".into())
            }
        }
    }

    fn disconnect_user(
        &mut self,
        address: &Address,
        user: &str,
        password: &str,
    ) -> Result<Vec<RoutedMessage>, String> {
        self.verify_user(address, user, password)?;
        let user = self.auth_users.get_mut(user).unwrap();
        if !user.is_connected_to_chat() {
            Err("not connected to chat".into())
        } else {
            let chat = user.connected_chat().unwrap().to_owned();
            user.disconnect_from_chat();

            let chat = self.chats.get_mut(chat.as_str()).unwrap();
            let connected_users = chat.connected_users();
            let event = chat.disconnect_user(user.name().into());

            let messages = self.prepare_event_broadcast(connected_users, event);

            Ok(messages)
        }
    }

    fn connect_user_to_chat(
        &mut self,
        chat: &str,
        address: &Address,
        user: &str,
        password: &str,
    ) -> Result<Vec<RoutedMessage>, String> {
        self.verify_user(address, user, password)?;

        let user = self.auth_users.get_mut(user).unwrap();
        if user.is_connected_to_chat() {
            Err("user already connected to chat".into())
        } else {
            let chat = self.chats.get_mut(chat);
            if chat.is_none() {
                Err("chat not found".into())
            } else {
                let chat = chat.unwrap();
                user.connect_to_chat(chat.name().into());

                let user_name = user.name().to_owned().clone();
                let user_address = self.auth_users.get(user_name.as_str()).unwrap().address();

                let previously_connected_users = chat.connected_users();

                let (event, history) = chat.connect_user(user_name.into());

                let history_message = self
                    .msg_builder
                    .new_chat_events(chat.name().into(), history);

                let history_message = RoutedMessage {
                    to: user_address,
                    msg: history_message,
                };

                let mut messages = self.prepare_event_broadcast(previously_connected_users, event);
                messages.push(history_message);
                Ok(messages)
            }
        }
    }

    fn user_send_message_in_chat(
        &mut self,
        address: &Address,
        user: &str,
        password: &str,
        message: String,
    ) -> Result<Vec<RoutedMessage>, String> {
        self.verify_user(address, user, password)?;
        let user = self.auth_users.get_mut(user).unwrap();
        if !user.is_connected_to_chat() {
            Err("not connected to chat".into())
        } else {
            let chat = user.connected_chat().unwrap();

            let chat = self.chats.get_mut(chat).unwrap();

            let chat_event = chat.send_message(user.name().to_owned(), message);
            let chat_users = chat.connected_users();
            let messages = self.prepare_event_broadcast(chat_users, chat_event);

            Ok(messages)
        }
    }

    fn user_create_chat(
        &mut self,
        address: &Address,
        user: &str,
        password: &str,
        chat_name: String,
    ) -> Result<Vec<RoutedMessage>, String> {
        self.verify_user(address, user, password)?;

        let user = self.auth_users.get_mut(user).unwrap();

        if user.is_connected_to_chat() {
            Err("user is connected to chat".into())
        } else {
            if self.chats.contains_key(chat_name.as_str()) {
                Err(format!("chat with name {:?} already exists", chat_name))
            } else {
                let user_name = user.name().to_owned();
                let (chat, _) = Chat::new(chat_name.clone(), user_name.clone());
                self.chats.insert(chat_name.clone(), chat);
                Ok(Vec::new())
            }
        }
    }

    fn prepare_event_broadcast(&self, users: Vec<String>, event: ChatEvent) -> Vec<RoutedMessage> {
        users
            .into_iter()
            .map(|user| {
                let addr = self.auth_users.get(user.as_str()).unwrap().address();
                RoutedMessage {
                    to: addr,
                    msg: self.msg_builder.new_chat_event(event.clone()),
                }
            })
            .collect()
    }
}
