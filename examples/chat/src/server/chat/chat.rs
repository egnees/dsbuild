//! Definition of chat history manager.

use std::collections::HashSet;

use dsbuild::{Address, Context};

use super::event::ChatEvent;

use crate::server::process::messages::ServerMessage;

/// Responsible for managing chat history and sending it to users.
pub struct Chat {
    pub chat_name: String,
    /// Name of file with history.
    history_file: String,
    pub history_size: usize,
    pub is_initialized: bool,
    pub connected_users: HashSet<String>,
}

impl Chat {
    /// New history manager.
    pub fn new(chat_name: String) -> Self {
        Self {
            history_file: chat_name.clone() + ".chat",
            chat_name,
            history_size: 0,
            is_initialized: false,
            connected_users: HashSet::new(),
        }
    }

    /// Make new record in chat history.
    pub async fn extend_history(&mut self, event: ChatEvent, ctx: Context) {
        if !self.is_initialized {
            panic!("history manager is not initialized")
        }

        let event = serde_json::to_string(&event).unwrap() + "\n";

        // Append event.
        ctx.append(&self.history_file, event.as_bytes())
            .await
            .unwrap();

        self.history_size += 1;
    }

    /// Transfer history to client with specified address.
    /// Returns true if the whole history has been sent without errors.
    pub async fn transfer_history_by_address(&mut self, address: Address, ctx: Context) {
        if !self.is_initialized {
            panic!("history manager is not initialized")
        }

        let mut offset = 0;
        let mut buf = vec![0; 4096];

        let mut current_event = Vec::new();

        loop {
            let read_bytes = ctx
                .read(&self.history_file, offset, buf.as_mut_slice())
                .await
                .unwrap();

            if read_bytes == 0 {
                break;
            }

            for c in buf.as_slice()[..read_bytes].iter() {
                if *c == b'\n' {
                    if !current_event.is_empty() {
                        // event done.
                        let event: ChatEvent =
                            serde_json::from_slice(&current_event.as_slice()).unwrap();

                        let chat_name = self.chat_name.clone();
                        let _ = ctx
                            .send_with_ack(
                                ServerMessage::ChatEvents(chat_name, vec![event]).into(),
                                address.clone(),
                                5.0,
                            )
                            .await;

                        current_event.clear();
                    }
                } else {
                    current_event.push(*c);
                }
            }

            offset += read_bytes;
        }

        if !current_event.is_empty() {
            let event: ChatEvent = serde_json::from_slice(&current_event.as_slice()).unwrap();

            let _ = ctx
                .send_with_ack(
                    ServerMessage::ChatEvents(self.chat_name.clone(), vec![event]).into(),
                    address.clone(),
                    5.0,
                )
                .await;
        }
    }

    /// Initializes history manager.
    pub async fn init(&mut self, ctx: Context) {
        let mut offset = 0;
        let mut buf = vec![0; 4096];

        if let Err(e) = ctx
            .read(&self.history_file, offset, buf.as_mut_slice())
            .await
        {
            match e {
                ReadError::FileNotFound => ctx.create_file(&self.history_file).await.unwrap(),
                _ => panic!("storage crashed"),
            }
        }

        let mut size = 0;

        loop {
            let read_bytes = ctx
                .read(&self.history_file, offset, buf.as_mut_slice())
                .await
                .unwrap();

            if read_bytes == 0 {
                break;
            }

            buf.as_slice()[..read_bytes].into_iter().for_each(|c| {
                if *c == b'\n' {
                    size += 1;
                }
            });

            offset += read_bytes;
        }

        self.is_initialized = true;
        self.history_size = size;
    }
}
