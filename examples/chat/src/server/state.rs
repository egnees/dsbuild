use std::{collections::HashMap, sync::Arc, time::SystemTime};

use dsbuild::{storage::StorageError, Address, Context};
use tokio::sync::Mutex;

use crate::client::requests::{ClientRequest, ClientRequestKind};

use super::{
    event::{ChatEvent, ChatEventKind},
    messages::ServerMessage,
    util::{auth_user, calc_events_in_chat, send_ack, send_err, transfer_chat_history},
};

#[derive(Default)]
pub struct ServerState {
    chat_seq_nums: HashMap<String, u64>,
    chat_users: HashMap<String, Vec<Address>>,
    user_chat: HashMap<String, String>,
}

pub type ServerStateLock = Arc<Mutex<ServerState>>;

impl ServerState {
    pub async fn handle_user_request(
        &mut self,
        from: Address,
        ctx: Context,
        request: ClientRequest,
    ) {
        let auth_result = auth_user(ctx.clone(), request.client.clone(), request.password).await;
        if !auth_result {
            send_err(ctx, request.id, from, "authentication failed".to_owned());
            return;
        }

        let result = match request.kind {
            ClientRequestKind::SendMessage(msg) => {
                self.send_message_to_chat(request.client, msg, ctx.clone())
                    .await
            }
            ClientRequestKind::Create(chat) => {
                self.create_chat(chat, request.client, from.clone(), ctx.clone())
                    .await
            }
            ClientRequestKind::Connect(chat) => {
                self.connect_to_chat(chat, request.client, from.clone(), ctx.clone())
                    .await
            }
            ClientRequestKind::Disconnect => {
                self.disconnect_from_chat(request.client, from.clone(), ctx.clone())
                    .await
            }
        };

        match result {
            Ok(_) => send_ack(ctx, request.id, from),
            Err(info) => send_err(ctx, request.id, from, info),
        }
    }

    async fn send_message_to_chat(
        &mut self,
        client: String,
        msg: String,
        ctx: Context,
    ) -> Result<(), String> {
        if msg.len() > 4096 {
            return Err("message too long".to_owned());
        }

        let chat = self
            .user_chat
            .get(&client)
            .ok_or("not connected to chat".to_owned())?
            .to_string();
        let event = self
            .append_event_to_chat(
                chat.clone(),
                client,
                ChatEventKind::SentMessage(msg),
                ctx.clone(),
            )
            .await?;
        self.broadcast_chat_event(chat, event, ctx);
        Ok(())
    }

    async fn create_chat(
        &mut self,
        chat: String,
        client: String,
        _client_addr: Address,
        ctx: Context,
    ) -> Result<(), String> {
        if chat.len() > 4096 {
            return Err("chat name too long".to_owned());
        }

        if !self.user_chat.get(&client).is_none() {
            return Err("already connected to chat".to_owned());
        }
        let file_name = format!("{}.chat", chat);
        let chat_exists = ctx.file_exists(&file_name).await.unwrap();
        if chat_exists {
            Err("chat already exists".to_owned())
        } else {
            ctx.create_file(&file_name).await.unwrap();
            self.chat_users.insert(chat.clone(), Vec::new());
            let event = self
                .append_event_to_chat(
                    chat.clone(),
                    client.clone(),
                    ChatEventKind::Created(),
                    ctx.clone(),
                )
                .await
                .unwrap();
            self.broadcast_chat_event(chat, event, ctx);
            Ok(())
        }
    }

    async fn connect_to_chat(
        &mut self,
        chat: String,
        client: String,
        client_addr: Address,
        ctx: Context,
    ) -> Result<(), String> {
        if let Some(_) = self.user_chat.get(&client) {
            Err("user already connected to chat".to_owned())
        } else {
            let event = self
                .append_event_to_chat(
                    chat.clone(),
                    client.clone(),
                    ChatEventKind::Connected(),
                    ctx.clone(),
                )
                .await?;
            self.connect_user_to_chat(chat.clone(), client_addr.clone(), client);
            transfer_chat_history(ctx.clone(), client_addr, chat.clone()).await;
            self.broadcast_chat_event(chat, event, ctx);
            Ok(())
        }
    }

    async fn disconnect_from_chat(
        &mut self,
        client: String,
        client_addr: Address,
        ctx: Context,
    ) -> Result<(), String> {
        let chat = self
            .user_chat
            .get(&client)
            .map(|s| s.to_string())
            .ok_or("user does not connected to the chat".to_owned())?;
        self.disconnect_user_from_chat(chat.to_string(), client_addr, client.clone());
        let event = self
            .append_event_to_chat(
                chat.clone(),
                client,
                ChatEventKind::Disconnected(),
                ctx.clone(),
            )
            .await?;
        self.broadcast_chat_event(chat, event, ctx);
        Ok(())
    }

    async fn append_event_to_chat(
        &mut self,
        chat: String,
        author: String,
        event: ChatEventKind,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
        let file_name = format!("{}.chat", chat);
        let mut file = ctx.open_file(&file_name).await.map_err(|e| match e {
            StorageError::NotFound => "chat not found".to_string(),
            _ => panic!("storage unavailable"),
        })?;
        let seq_num = *self
            .chat_seq_nums
            .get(&chat)
            .unwrap_or(&calc_events_in_chat(ctx.clone(), chat.clone()).await);
        self.chat_seq_nums.insert(chat.clone(), seq_num + 1);
        let event = ChatEvent {
            chat,
            user: author,
            time: SystemTime::now(),
            kind: event,
            seq: seq_num,
        };
        let event_data = serde_json::to_string(&event).unwrap() + "\n";
        let event_data_bytes = event_data.as_bytes();
        let mut offset = 0;
        loop {
            let appended = file
                .append(&event_data_bytes[offset as usize..])
                .await
                .unwrap();
            if appended == 0 {
                break;
            }
            offset += appended;
        }

        Ok(event)
    }

    fn connect_user_to_chat(&mut self, chat: String, user_addr: Address, user: String) {
        self.chat_users
            .entry(chat.clone())
            .or_insert(Vec::new())
            .push(user_addr);
        assert!(self.user_chat.insert(user, chat).is_none());
    }

    fn disconnect_user_from_chat(&mut self, chat: String, user_addr: Address, user: String) {
        self.chat_users
            .get_mut(&chat)
            .unwrap()
            .retain(|addr| *addr != user_addr);
        self.user_chat.remove(&user).unwrap();
    }

    fn broadcast_chat_event(&self, chat: String, event: ChatEvent, ctx: Context) {
        let users = self.chat_users.get(&chat).unwrap();
        for user in users {
            let msg = ServerMessage::ChatEvent(chat.clone(), event.clone()).into();
            let to = user.clone();
            let ctx_clone = ctx.clone();
            ctx.clone().spawn(async move {
                let _ = ctx_clone.send_with_ack(msg, to, 5.0).await;
            });
        }
    }
}
