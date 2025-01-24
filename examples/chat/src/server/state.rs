use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use dsbuild::{Address, Context, Message};
use tokio::sync::Mutex;

use crate::{
    client::requests::{ClientRequest, ClientRequestKind},
    server::util::get_client_address,
};

use super::{
    event::{ChatEvent, ChatEventKind},
    messages::ServerMessage,
    replication::{
        get_replica_total_seq_num, replicate_client_request, request_replica_events_from_range,
        ReceiveEventsRequest, ReplicateRequest, TotalSeqNumMsg, TotalSeqNumRequest,
    },
    util::{
        append_client_request, append_event_to_chat_history, append_event_to_client_history,
        auth_user, calc_events_in_chat, calc_global_requests_cnt, chat_exists, get_client_chat,
        get_clients_connected_to_chat, send_ack, send_info, transfer_chat_history,
        transfer_requests,
    },
};

#[derive(Default)]
pub struct ServerState {
    chat_seq_nums: HashMap<String, u64>,
    chat_users: HashMap<String, Vec<Address>>,
    user_chat: HashMap<String, Option<String>>,
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

    pub async fn get_users_connected_to_chat(
        &mut self,
        ctx: Context,
        chat: String,
    ) -> Vec<Address> {
        if let Entry::Vacant(e) = self.chat_users.entry(chat.clone()) {
            let users = get_clients_connected_to_chat(ctx.clone(), chat).await;
            let mut addrs = Vec::new();
            for user in users.into_iter() {
                let addr = get_client_address(ctx.clone(), user).await.unwrap();
                addrs.push(addr);
            }
            e.insert(addrs.clone());
            addrs
        } else {
            self.chat_users.get(&chat).unwrap().to_owned()
        }
    }

    pub async fn get_user_chat(&mut self, ctx: Context, user: String) -> Option<String> {
        if let Entry::Vacant(e) = self.user_chat.entry(user.clone()) {
            let chat = get_client_chat(ctx.clone(), user).await;
            e.insert(chat.clone());
            chat
        } else {
            self.user_chat.get(&user).unwrap().to_owned()
        }
    }

    async fn get_total_seq(&mut self, ctx: Context) -> u64 {
        self.total_seq
            .get_or_insert(calc_global_requests_cnt(ctx).await)
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
        let tip = msg.tip().as_str();
        match tip {
            "total_seq_num_request" => {
                let tag = msg.data::<TotalSeqNumRequest>().unwrap().tag;
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
            "replicate_request" => {
                let request = msg.data::<ReplicateRequest>().unwrap();
                let seq = self.get_total_seq(ctx.clone()).await;
                if seq <= request.seq_num {
                    assert_eq!(seq, request.seq_num);
                    if let Some(waiting_for) = self.waiting_for_seq {
                        if waiting_for == request.seq_num {
                            self.waiting_for_seq = None;
                        }
                    }
                    self.handle_user_request(from, ctx, request.client_request)
                        .await;
                }
            }
            "total_seq_num_msg" => {
                // do nothing
            }
            "receive_events_request" => {
                let request = msg.data::<ReceiveEventsRequest>().unwrap();
                transfer_requests(ctx.clone(), from, request.from, request.to).await;
            }
            "client_request" => {
                let user_request = msg.data::<ClientRequest>().unwrap();
                if !self.check_replication(ctx.clone()).await {
                    return;
                }
                self.handle_user_request(from, ctx, user_request).await;
            }
            _ => log::warn!("got unexpected message tip: {:?}", tip),
        }
    }

    pub async fn get_chat_seq_num(&mut self, ctx: Context, chat: String) -> u64 {
        *self
            .chat_seq_nums
            .entry(chat.clone())
            .or_insert(calc_events_in_chat(ctx.clone(), chat).await)
    }

    async fn connect_user_to_chat(&mut self, ctx: Context, chat: String, user: String) {
        self.user_chat.insert(user.clone(), Some(chat.clone()));
        let addr = get_client_address(ctx, user).await.unwrap();
        self.chat_users.entry(chat).or_default().push(addr);
    }

    async fn disconnect_user_from_chat(&mut self, ctx: Context, chat: String, user: String) {
        self.user_chat.insert(user.clone(), None);
        let addr = get_client_address(ctx.clone(), user).await.unwrap();
        let _ = self.get_users_connected_to_chat(ctx, chat.clone()).await;
        self.chat_users
            .get_mut(&chat)
            .unwrap()
            .retain(|user_addr| *user_addr != addr);
    }

    async fn broadcast_chat_event(&mut self, chat: String, event: ChatEvent, ctx: Context) {
        let users = self
            .get_users_connected_to_chat(ctx.clone(), chat.clone())
            .await;
        for user in users {
            let msg = ServerMessage::ChatEvent(chat.clone(), event.clone()).into();
            let to = user.clone();
            let ctx_clone = ctx.clone();
            ctx.clone().spawn(async move {
                let _ = ctx_clone.send_with_ack(msg, to, 5.0).await;
            });
        }
    }

    async fn client_status(&mut self, client: String, ctx: Context) -> String {
        if let Some(chat) = self.get_user_chat(ctx.clone(), client.clone()).await {
            let addr = get_client_address(ctx.clone(), client).await.unwrap();
            transfer_chat_history(ctx, addr, chat.clone()).await;
            chat
        } else {
            String::new()
        }
    }

    pub async fn handle_user_request(
        &mut self,
        from: Address,
        ctx: Context,
        mut request: ClientRequest,
    ) {
        let from_replica = self.replica.is_some() && *self.replica.as_ref().unwrap() == from;
        if !from_replica {
            request.addr = Some(from.clone());
            request.time = Some(ctx.time());
        }

        let client_addr = request.addr.clone().unwrap();

        // Auth user
        let auth_result = auth_user(
            ctx.clone(),
            request.client.clone(),
            request.password.clone(),
            client_addr.clone(),
        )
        .await;

        if !auth_result {
            if !from_replica {
                send_info(
                    ctx,
                    request.id,
                    client_addr,
                    "authentication failed".to_owned(),
                );
            } else {
                log::error!("incorrect replication request from replica");
            }
            return;
        }

        let event_time = request.time.unwrap();
        let result = match request.kind.clone() {
            ClientRequestKind::SendMessage(msg) => {
                self.send_message_to_chat(request.client.clone(), msg, ctx.clone(), event_time)
                    .await
            }
            ClientRequestKind::Create(chat) => {
                self.create_chat(chat, request.client.clone(), ctx.clone(), event_time)
                    .await
            }
            ClientRequestKind::Connect(chat) => {
                self.connect_to_chat(chat, request.client.clone(), ctx.clone(), event_time)
                    .await
            }
            ClientRequestKind::Disconnect => {
                self.disconnect_from_chat(request.client.clone(), ctx.clone(), event_time)
                    .await
            }
            ClientRequestKind::Status => Err(self
                .client_status(request.client.clone(), ctx.clone())
                .await),
        };

        let request_id = request.id;
        match result {
            Ok(()) => {
                append_client_request(ctx.clone(), request.clone()).await;
                let seq = self.get_total_seq(ctx.clone()).await;
                if let Some(replica) = self.replica.clone() {
                    if !from_replica {
                        request.addr = Some(client_addr.clone());
                        replicate_client_request(ctx.clone(), replica, request, seq).await;
                    }
                }
                *self.total_seq.as_mut().unwrap() = seq + 1;
                if !from_replica {
                    send_ack(ctx, request_id, client_addr);
                }
            }
            Err(info) => {
                if !from_replica {
                    send_info(ctx, request_id, client_addr, info);
                } else {
                    log::error!("incorrect replication request from replica: {:?}", info);
                }
            }
        }
    }

    async fn send_message_to_chat(
        &mut self,
        client: String,
        msg: String,
        ctx: Context,
        time: f64,
    ) -> Result<(), String> {
        if msg.len() > 4096 {
            return Err("message too long".to_owned());
        }

        let chat = self
            .get_user_chat(ctx.clone(), client.clone())
            .await
            .ok_or("user not connected to chat".to_string())?;

        let event = self
            .apply_chat_event_from_user(
                chat.clone(),
                client,
                ChatEventKind::SentMessage(msg),
                time,
                ctx.clone(),
            )
            .await;

        self.broadcast_chat_event(chat, event, ctx).await;

        Ok(())
    }

    async fn create_chat(
        &mut self,
        chat: String,
        client: String,
        ctx: Context,
        time: f64,
    ) -> Result<(), String> {
        if chat.len() > 4096 {
            return Err("chat name too long".to_owned());
        }

        if self
            .get_user_chat(ctx.clone(), client.clone())
            .await
            .is_some()
        {
            return Err("already connected to chat".to_owned());
        }

        if chat_exists(ctx.clone(), chat.clone()).await {
            return Err("chat already exists".to_owned());
        }

        let event = self
            .apply_chat_event_from_user(
                chat.clone(),
                client,
                ChatEventKind::Created(),
                time,
                ctx.clone(),
            )
            .await;

        self.broadcast_chat_event(chat, event.clone(), ctx).await;

        Ok(())
    }

    async fn connect_to_chat(
        &mut self,
        chat: String,
        client: String,
        ctx: Context,
        time: f64,
    ) -> Result<(), String> {
        if !chat_exists(ctx.clone(), chat.clone()).await {
            return Err("chat with such name does not exist".to_string());
        }

        if self
            .get_user_chat(ctx.clone(), client.clone())
            .await
            .is_some()
        {
            return Err("user already connected to chat".to_string());
        }

        self.connect_user_to_chat(ctx.clone(), chat.clone(), client.clone())
            .await;
        let event = self
            .apply_chat_event_from_user(
                chat.clone(),
                client.clone(),
                ChatEventKind::Connected(),
                time,
                ctx.clone(),
            )
            .await;
        let to = get_client_address(ctx.clone(), client).await.unwrap();
        transfer_chat_history(ctx.clone(), to, chat.clone()).await;
        self.broadcast_chat_event(chat, event, ctx).await;

        Ok(())
    }

    async fn disconnect_from_chat(
        &mut self,
        client: String,
        ctx: Context,
        time: f64,
    ) -> Result<(), String> {
        let chat = self
            .get_user_chat(ctx.clone(), client.clone())
            .await
            .ok_or("user not connected to chat".to_string())?;

        self.disconnect_user_from_chat(ctx.clone(), chat.clone(), client.clone())
            .await;
        let event = self
            .apply_chat_event_from_user(
                chat.clone(),
                client.clone(),
                ChatEventKind::Disconnected(),
                time,
                ctx.clone(),
            )
            .await;
        self.broadcast_chat_event(chat, event, ctx).await;

        Ok(())
    }

    // here chat files can be absent
    pub async fn apply_chat_event_from_user(
        &mut self,
        chat: String,
        author: String,
        event_kind: ChatEventKind,
        event_time: f64,
        ctx: Context,
    ) -> ChatEvent {
        let chat_seq_num = self.get_chat_seq_num(ctx.clone(), chat.clone()).await;
        self.chat_seq_nums.insert(chat.clone(), chat_seq_num + 1);
        let event = ChatEvent {
            chat: chat.clone(),
            user: author.clone(),
            time: event_time,
            kind: event_kind,
            seq: chat_seq_num,
        };

        append_event_to_client_history(ctx.clone(), author, event.clone()).await;
        append_event_to_chat_history(ctx, chat, event.clone()).await;

        event
    }
}
