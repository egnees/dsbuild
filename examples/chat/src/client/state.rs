//! Definition of possible client states and related logic.

use std::collections::VecDeque;

use crate::server::messages::ServerMessage;

use super::{
    chat::Chat,
    io::Info,
    requests::{ClientRequest, ClientRequestKind},
};

/// Represents possible states of client.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum WaitingFor {
    /// Corresponds to state,
    /// when client listens for the user requests or server messages.
    ClientRequestOrServerMessage,
    /// Corresponds to state,
    /// when client waits for the response of previous request.
    /// Id of request is specified.
    ServerResponse(u64, ClientRequestKind),
}

impl Default for WaitingFor {
    fn default() -> Self {
        Self::ClientRequestOrServerMessage
    }
}

/// Represents information which [`State`] returns to the client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateUpdateResult {
    /// Request which client can send to the server.
    /// After one update of [`State`] client can send at most one message,
    /// because client need to wait server response,
    /// and then update [`State`] with it.
    pub to_server: Option<ClientRequest>,
    /// Information which client can send to the user in the same order.
    pub to_user: Vec<Info>,
}

impl StateUpdateResult {
    pub fn from_nothing() -> Self {
        Self {
            to_server: None,
            to_user: Vec::new(),
        }
    }

    pub fn from_to_server_request(request: ClientRequest) -> Self {
        Self {
            to_server: Some(request),
            to_user: Vec::new(),
        }
    }

    pub fn from_to_user_info(info: Info) -> Self {
        Self {
            to_server: None,
            to_user: vec![info],
        }
    }

    pub fn from_to_user_info_vec(info: Vec<Info>) -> Self {
        Self {
            to_server: None,
            to_user: info,
        }
    }

    pub fn add_to_user_info(mut self, info: Info) -> Self {
        self.to_user.push(info);
        self
    }

    pub fn add_to_user_info_vec(mut self, mut info: Vec<Info>) -> Self {
        self.to_user.append(&mut info);
        self
    }

    pub fn set_to_server_request(&mut self, request: ClientRequest) {
        self.to_server = Some(request);
    }
}

/// Represents state of client.
#[derive(Default, Debug, Clone)]
pub struct State {
    chat: Option<Chat>,
    waiting_for: WaitingFor,
    pending_client_requests: VecDeque<ClientRequest>,
    pending_server_messages: Vec<ServerMessage>,
}

impl State {
    /// Apply specified client request.
    ///
    /// # Returns
    ///
    /// * Ok(client_request) which can be send to server.
    /// * Err(Option(info)) means it can not be send now (and may be sent after), with optional error message.
    pub fn apply_client_request(&mut self, request: ClientRequest) -> StateUpdateResult {
        match self.waiting_for {
            WaitingFor::ClientRequestOrServerMessage => {
                self.waiting_for = WaitingFor::ServerResponse(request.id, request.kind.clone());
                StateUpdateResult::from_to_server_request(request)
            }
            WaitingFor::ServerResponse(_, _) => {
                self.pending_client_requests.push_back(request);
                StateUpdateResult::from_nothing()
            }
        }
    }

    /// Apply specified server message.
    ///
    /// # Returns
    ///
    /// Info about state which can be shown to the user.
    pub fn apply_server_msg(&mut self, msg: ServerMessage) -> StateUpdateResult {
        match &self.waiting_for {
            WaitingFor::ClientRequestOrServerMessage => {
                match msg {
                    ServerMessage::RequestResponse(_, _) => StateUpdateResult::from_nothing(), // outdated response
                    ServerMessage::ChatEvent(chat, event) => {
                        match &mut self.chat {
                            Some(current_chat) => {
                                if current_chat.name() == chat {
                                    let events_info = current_chat
                                        .process_event(event)
                                        .into_iter()
                                        .map(|event| event.into())
                                        .collect();
                                    StateUpdateResult::from_to_user_info_vec(events_info)
                                } else {
                                    StateUpdateResult::from_nothing() // outdated chat events
                                }
                            }
                            None => StateUpdateResult::from_nothing(), // outdated chat events
                        }
                    }
                }
            }
            WaitingFor::ServerResponse(waiting_request_id, waiting_request_kind) => {
                match &msg {
                    ServerMessage::ChatEvent(_, _) => {
                        self.pending_server_messages.push(msg);
                        StateUpdateResult::from_nothing()
                    }
                    ServerMessage::RequestResponse(got_request_id, request_result) => {
                        if *waiting_request_id != *got_request_id {
                            StateUpdateResult::from_nothing() // Ignore.
                        } else {
                            self.on_request_responded(
                                waiting_request_kind.clone(),
                                request_result.clone(),
                            )
                        }
                    }
                }
            }
        }
    }

    /// Handle response on request for which state was waiting.
    fn on_request_responded(
        &mut self,
        request_kind: ClientRequestKind,
        request_result: Result<(), String>,
    ) -> StateUpdateResult {
        self.waiting_for = WaitingFor::ClientRequestOrServerMessage;

        let mut update_result = match request_kind {
            ClientRequestKind::SendMessage(_) => match request_result {
                Ok(_) => StateUpdateResult::from_to_user_info_vec(
                    self.drain_and_filter_pending_server_messages(),
                ),
                Err(info) => StateUpdateResult::from_to_user_info(info.as_str().into())
                    .add_to_user_info_vec(self.drain_and_filter_pending_server_messages()),
            },
            ClientRequestKind::Create(_) => match request_result {
                Ok(_) => StateUpdateResult::from_to_user_info_vec(
                    self.drain_and_filter_pending_server_messages(),
                ),
                Err(info) => StateUpdateResult::from_to_user_info(info.as_str().into())
                    .add_to_user_info_vec(self.drain_and_filter_pending_server_messages()),
            },
            ClientRequestKind::Connect(connected_chat) => match request_result {
                Ok(_) => {
                    self.chat = Some(Chat::new(connected_chat));
                    StateUpdateResult::from_to_user_info_vec(
                        self.drain_and_filter_pending_server_messages(),
                    )
                }
                Err(info) => StateUpdateResult::from_to_user_info(info.as_str().into())
                    .add_to_user_info_vec(self.drain_and_filter_pending_server_messages()),
            },
            ClientRequestKind::Disconnect => match request_result {
                Ok(_) => {
                    self.chat = None;
                    StateUpdateResult::from_to_user_info_vec(
                        self.drain_and_filter_pending_server_messages(),
                    )
                }
                Err(info) => StateUpdateResult::from_to_user_info(info.as_str().into())
                    .add_to_user_info_vec(self.drain_and_filter_pending_server_messages()),
            },
            ClientRequestKind::Status => match request_result {
                Ok(_) => {
                    panic!("there should not be ok response on status request")
                }
                Err(connected_chat) => {
                    let info = if connected_chat.is_empty() {
                        "user not connected to chat".to_string()
                    } else {
                        format!("user connected to {}", connected_chat)
                    };
                    if self.chat.is_some() {
                        assert_eq!(self.chat.as_ref().unwrap().name(), connected_chat);
                    } else {
                        self.chat = Some(Chat::new(connected_chat));
                    }
                    StateUpdateResult::from_to_user_info(info.as_str().into())
                        .add_to_user_info_vec(self.drain_and_filter_pending_server_messages())
                }
            },
        };

        if self.waiting_for == WaitingFor::ClientRequestOrServerMessage {
            if let Some(client_request) = self.pending_client_requests.pop_front() {
                self.waiting_for =
                    WaitingFor::ServerResponse(client_request.id, client_request.kind.clone());
                update_result.set_to_server_request(client_request);
            }
        }

        update_result
    }

    fn drain_and_filter_pending_server_messages(&mut self) -> Vec<Info> {
        if let Some(current_chat) = &mut self.chat {
            let mut result = Vec::new();

            let drain = self.pending_server_messages.drain(..);

            for message in drain {
                match message {
                    ServerMessage::ChatEvent(destination_chat, event) => {
                        if current_chat.name() == destination_chat {
                            let mut events_info: Vec<Info> = current_chat
                                .process_event(event)
                                .into_iter()
                                .map(|event| event.into())
                                .collect();

                            result.append(&mut events_info);
                        }
                    }
                    _ => panic!("there can not be responses on requests in the pending messages"),
                }
            }

            result
        } else {
            self.pending_server_messages.clear();
            Vec::<Info>::new()
        }
    }
}
