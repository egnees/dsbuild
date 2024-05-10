use std::{collections::HashMap, sync::Arc, time::SystemTime};

use dsbuild::{storage::StorageError, Address, Context, Message};
use tokio::sync::Mutex;

use crate::client::requests::{ClientRequest, ClientRequestKind};

use super::{
    event::{ChatEvent, ChatEventKind},
    messages::ServerMessage,
    replication::{
        get_replica_total_seq_num, replicate_event, request_replica_events_from_range,
        ReceiveEventsRequest, ReplicateEventRequest, TotalSeqNumMsg, TotalSeqNumRequest,
    },
    util::{
        append_global_event, auth_user, calc_events_in_chat, calc_global_events_cnt, send_ack,
        send_err, transfer_chat_history, transfer_events,
    },
};

#[derive(Default)]
pub struct ServerState {
    chat_seq_nums: HashMap<String, u64>,
    chat_users: HashMap<String, Vec<Address>>,
    user_chat: HashMap<String, String>,
    waiting_for_seq: Option<u64>,
    total_seq: Option<u64>,
    replica: Option<Address>,
    tag: u64,
}

pub type ServerStateLock = Arc<Mutex<ServerState>>;

impl ServerState {
    pub fn new_with_replica(replica: Address) -> Self {
        Self {
            chat_seq_nums: HashMap::new(),
            chat_users: HashMap::new(),
            user_chat: HashMap::new(),
            waiting_for_seq: None,
            total_seq: None,
            replica: Some(replica),
            tag: 0,
        }
    }

    async fn get_total_seq(&mut self, ctx: Context) -> u64 {
        self.total_seq
            .get_or_insert(calc_global_events_cnt(ctx).await)
            .to_owned()
    }

    pub async fn check_replication(&mut self, ctx: Context) -> bool {
        if self.waiting_for_seq.is_some() {
            return false;
        }

        if let Some(replica) = self.replica.clone() {
            self.tag += 1;
            let replica_seq =
                get_replica_total_seq_num(ctx.clone(), self.tag, replica.clone()).await;
            let seq = self.get_total_seq(ctx.clone()).await;
            if let Some(replica_seq) = replica_seq {
                if seq < replica_seq {
                    request_replica_events_from_range(ctx.clone(), seq, replica_seq - 1, replica)
                        .await;
                    self.waiting_for_seq = Some(replica_seq - 1);
                    return false;
                }
            }
        }
        true
    }

    pub async fn process_msg(&mut self, from: Address, ctx: Context, msg: Message) {
        let tip = msg.get_tip().as_str();
        match tip {
            "total_seq_num_request" => {
                let tag = msg.get_data::<TotalSeqNumRequest>().unwrap().tag;
                let total_seq = self.get_total_seq(ctx.clone()).await;
                let _ = ctx
                    .send_with_tag(
                        TotalSeqNumMsg {
                            total_seq_num: total_seq,
                        }
                        .into(),
                        tag,
                        from,
                        5.0,
                    )
                    .await;
            }
            "replicate_event_request" => {
                let request = msg.get_data::<ReplicateEventRequest>().unwrap();
                let seq = self.get_total_seq(ctx.clone()).await;
                if seq <= request.total_seq_num {
                    assert_eq!(seq, request.total_seq_num);
                    self.total_seq = Some(seq + 1);
                    if let Some(waiting_for) = self.waiting_for_seq {
                        if waiting_for == request.total_seq_num {
                            self.waiting_for_seq = None;
                        }
                    }
                    append_global_event(ctx.clone(), request.event.clone()).await;
                    let _ = self
                        .append_event_to_chat(
                            request.event.chat,
                            request.event.user,
                            request.event.kind,
                            ctx,
                        )
                        .await;
                }
            }
            "total_seq_num_msg" => {
                // do nothing
            }
            "receive_events_request" => {
                let request = msg.get_data::<ReceiveEventsRequest>().unwrap();
                transfer_events(ctx.clone(), from, request.from, request.to).await;
            }
            _ => {
                let user_request = msg.get_data::<ClientRequest>().unwrap();
                if !self.check_replication(ctx.clone()).await {
                    return;
                }
                self.handle_user_request(from, ctx, user_request).await;
            }
        }
    }

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
            Ok(event) => {
                append_global_event(ctx.clone(), event.clone()).await;
                let seq = self.get_total_seq(ctx.clone()).await;
                if let Some(replica) = self.replica.clone() {
                    replicate_event(ctx.clone(), replica, event, seq).await;
                }
                *self.total_seq.as_mut().unwrap() = seq + 1;
                send_ack(ctx, request.id, from);
            }
            Err(info) => send_err(ctx, request.id, from, info),
        }
    }

    async fn send_message_to_chat(
        &mut self,
        client: String,
        msg: String,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
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
        self.broadcast_chat_event(chat, event.clone(), ctx);
        Ok(event)
    }

    async fn create_chat(
        &mut self,
        chat: String,
        client: String,
        _client_addr: Address,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
        if chat.len() > 4096 {
            return Err("chat name too long".to_owned());
        }

        if self.user_chat.contains_key(&client) {
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
            self.broadcast_chat_event(chat, event.clone(), ctx);
            Ok(event)
        }
    }

    async fn connect_to_chat(
        &mut self,
        chat: String,
        client: String,
        client_addr: Address,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
        if self.user_chat.contains_key(&client) {
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
            self.broadcast_chat_event(chat, event.clone(), ctx);
            Ok(event)
        }
    }

    async fn disconnect_from_chat(
        &mut self,
        client: String,
        client_addr: Address,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
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
        self.broadcast_chat_event(chat, event.clone(), ctx);
        Ok(event)
    }

    pub async fn append_event_to_chat(
        &mut self,
        chat: String,
        author: String,
        event: ChatEventKind,
        ctx: Context,
    ) -> Result<ChatEvent, String> {
        let file_name = format!("{}.chat", chat);

        let mut file = if !ctx
            .file_exists(&file_name)
            .await
            .map_err(|_| "storage unavailable")?
        {
            ctx.create_file(&file_name)
                .await
                .map_err(|_| "storage unavailable")?
        } else {
            ctx.open_file(&file_name).await.map_err(|e| match e {
                StorageError::NotFound => "chat not found".to_string(),
                _ => panic!("storage unavailable"),
            })?
        };

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
            .or_default()
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
