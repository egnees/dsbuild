use std::collections::HashSet;

use dsbuild::{storage::StorageError, Address, Context};

use crate::{client::requests::ClientRequest, server::event::ChatEventKind};

use super::{event::ChatEvent, messages::ServerMessage, replication::ReplicateRequest};

pub fn send_info(ctx: Context, request_id: u64, to: Address, info: String) {
    ctx.clone().spawn(async move {
        let _ = ctx
            .send_with_ack(
                ServerMessage::RequestResponse(request_id, Err(info)).into(),
                to,
                5.0,
            )
            .await;
    });
}

pub fn send_ack(ctx: Context, request_id: u64, to: Address) {
    ctx.clone().spawn(async move {
        let _ = ctx
            .send_with_ack(
                ServerMessage::RequestResponse(request_id, Ok(())).into(),
                to,
                5.0,
            )
            .await;
    });
}

pub async fn auth_user(ctx: Context, login: String, password: String, addr: Address) -> bool {
    let pass_file_name = get_client_password_file_name(login.clone());
    let addr_file_name = get_client_address_file_name(login.clone());
    let user_registered = ctx.file_exists(&pass_file_name).await.unwrap();
    if !user_registered {
        let mut file = ctx.create_file(&pass_file_name).await.unwrap();
        let bytes_write = file.append(password.as_bytes()).await.unwrap();
        assert_eq!(bytes_write, password.len() as u64);

        let mut file = ctx.create_file(&addr_file_name).await.unwrap();
        let addr_ser = serde_json::to_vec(&addr).unwrap();
        file.append(&addr_ser).await.unwrap();

        true
    } else {
        let mut file = ctx.open_file(&pass_file_name).await.unwrap();
        let mut data = vec![0u8; 4096];
        let read_bytes = file.read(0, data.as_mut_slice()).await.unwrap();
        if password.as_bytes() != &data.as_slice()[..read_bytes as usize] {
            return false;
        }

        let mut file = ctx.open_file(&addr_file_name).await.unwrap();
        let mut data = vec![0u8; 4096];
        let read_bytes = file.read(0, data.as_mut_slice()).await.unwrap();
        let real_addr: Address =
            serde_json::from_slice(&data.as_slice()[..read_bytes as usize]).unwrap();
        if real_addr != addr {
            return false;
        }

        true
    }
}

pub async fn transfer_chat_history(ctx: Context, to: Address, chat: String) {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = get_chat_history_file_name(chat.clone());
    let mut file = ctx.open_file(&file_name).await.unwrap();

    let mut current_event = Vec::new();

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();

        if read_bytes == 0 {
            break;
        }

        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_event.is_empty() {
                    // event done.
                    let event: ChatEvent =
                        serde_json::from_slice(current_event.as_slice()).unwrap();

                    let msg = ServerMessage::ChatEvent(chat.clone(), event).into();
                    let to = to.clone();
                    let ctx_clone = ctx.clone();
                    ctx.clone().spawn(async move {
                        let _ = ctx_clone.send_with_ack(msg, to, 5.0).await;
                    });
                    current_event.clear();
                }
            } else {
                current_event.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_event.is_empty() {
        let event: ChatEvent = serde_json::from_slice(current_event.as_slice()).unwrap();

        let msg = ServerMessage::ChatEvent(chat, event).into();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(msg, to, 5.0).await;
        });
    }
}

pub async fn calc_events_in_chat(ctx: Context, chat: String) -> u64 {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = get_chat_history_file_name(chat);
    let mut file = if ctx.file_exists(&file_name).await.unwrap() {
        ctx.open_file(&file_name).await.unwrap()
    } else {
        ctx.create_file(&file_name).await.unwrap()
    };

    let mut current_event = Vec::new();
    let mut cnt = 0;

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();

        if read_bytes == 0 {
            break;
        }

        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_event.is_empty() {
                    cnt += 1;
                    current_event.clear();
                }
            } else {
                current_event.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_event.is_empty() {
        cnt += 1;
    }

    cnt
}

pub async fn append_client_request(ctx: Context, client_request: ClientRequest) {
    let file_name = &get_global_requests_file_name();
    let mut data = serde_json::to_vec(&client_request).unwrap();
    data.push(b'\n');
    let bytes = if !ctx.file_exists(file_name).await.unwrap() {
        ctx.create_file(file_name).await.unwrap()
    } else {
        ctx.open_file(file_name).await.unwrap()
    }
    .append(data.as_slice())
    .await
    .unwrap();

    if bytes != data.len() as u64 {
        panic!("storage unavailable");
    }
}

pub async fn calc_global_requests_cnt(ctx: Context) -> u64 {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = &get_global_requests_file_name();
    let mut file = if !ctx.file_exists(file_name).await.unwrap() {
        ctx.create_file(file_name).await.unwrap()
    } else {
        ctx.open_file(file_name).await.unwrap()
    };

    let mut current_event = Vec::new();
    let mut cnt = 0;

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();

        if read_bytes == 0 {
            break;
        }

        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_event.is_empty() {
                    cnt += 1;
                    current_event.clear();
                }
            } else {
                current_event.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_event.is_empty() {
        cnt += 1;
    }

    cnt
}

pub async fn transfer_requests(ctx: Context, to: Address, range_from: u64, range_to: u64) {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = &get_global_requests_file_name();
    let mut file = ctx.open_file(file_name).await.unwrap();

    let mut current_request = Vec::new();
    let mut cnt = 0;

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();

        if read_bytes == 0 {
            break;
        }

        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_request.is_empty() {
                    if range_from <= cnt && cnt <= range_to {
                        let client_request: ClientRequest =
                            serde_json::from_slice(current_request.as_slice()).unwrap();

                        let msg = ReplicateRequest {
                            seq_num: cnt,
                            client_request,
                        }
                        .into();
                        let to = to.clone();
                        let result = ctx.send_with_ack(msg, to, 5.0).await;
                        if result.is_err() {
                            return;
                        }
                    }
                    current_request.clear();
                    cnt += 1;
                }
            } else {
                current_request.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_request.is_empty() {
        let client_request: ClientRequest =
            serde_json::from_slice(current_request.as_slice()).unwrap();

        let msg = ReplicateRequest {
            seq_num: cnt,
            client_request,
        }
        .into();
        let _ = ctx.send_with_ack(msg, to, 5.0).await;
    }
}

pub async fn get_client_chat(ctx: Context, client: String) -> Option<String> {
    let file_name = get_client_history_file_name(client);
    let mut file = ctx
        .open_file(&file_name)
        .await
        .map_err(|e| match e {
            StorageError::NotFound => e,
            _ => panic!("storage unavailable"),
        })
        .ok()?;
    let mut buf = vec![0u8; 4096];

    let mut current_event = Vec::new();
    let mut offset = 0;

    let mut chat_opt: Option<String> = None;

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();
        if read_bytes == 0 {
            break;
        }
        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_event.is_empty() {
                    let event: ChatEvent =
                        serde_json::from_slice(current_event.as_slice()).unwrap();
                    match event.kind {
                        ChatEventKind::Connected() => chat_opt = Some(event.chat),
                        ChatEventKind::Disconnected() => chat_opt = None,
                        _ => {}
                    }
                    current_event.clear();
                }
            } else {
                current_event.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_event.is_empty() {
        let event: ChatEvent = serde_json::from_slice(current_event.as_slice()).unwrap();

        match event.kind {
            ChatEventKind::Connected() => chat_opt = Some(event.chat),
            ChatEventKind::Disconnected() => chat_opt = None,
            _ => {}
        }
    }

    chat_opt
}

pub async fn append_event_to_client_history(ctx: Context, client: String, event: ChatEvent) {
    let file_name = get_client_history_file_name(client);
    let mut data = serde_json::to_vec(&event).unwrap();
    data.push(b'\n');
    let bytes = if !ctx.file_exists(&file_name).await.unwrap() {
        ctx.create_file(&file_name).await.unwrap()
    } else {
        ctx.open_file(&file_name).await.unwrap()
    }
    .append(data.as_slice())
    .await
    .unwrap();

    if bytes != data.len() as u64 {
        panic!("storage unavailable");
    }
}

pub async fn append_event_to_chat_history(ctx: Context, chat: String, event: ChatEvent) {
    let file_name = get_chat_history_file_name(chat.clone());
    let mut data = serde_json::to_vec(&event).unwrap();
    data.push(b'\n');
    let bytes = if !ctx.file_exists(&file_name).await.unwrap() {
        ctx.create_file(&file_name).await.unwrap()
    } else {
        ctx.open_file(&file_name).await.unwrap()
    }
    .append(data.as_slice())
    .await
    .unwrap();

    if bytes != data.len() as u64 {
        panic!("storage unavailable");
    }
}

pub async fn get_clients_connected_to_chat(ctx: Context, chat: String) -> Vec<String> {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = get_chat_history_file_name(chat.clone());
    let mut file = ctx.open_file(&file_name).await.unwrap();

    let mut current_event = Vec::new();

    let mut connected_users = HashSet::new();

    loop {
        let read_bytes = file.read(offset, buf.as_mut_slice()).await.unwrap();

        if read_bytes == 0 {
            break;
        }

        for c in buf.as_slice()[..read_bytes as usize].iter() {
            if *c == b'\n' {
                if !current_event.is_empty() {
                    let event: ChatEvent =
                        serde_json::from_slice(current_event.as_slice()).unwrap();

                    match event.kind {
                        ChatEventKind::Connected() => {
                            connected_users.insert(event.user);
                        }
                        ChatEventKind::Disconnected() => {
                            connected_users.remove(&event.user);
                        }
                        _ => {}
                    }
                    current_event.clear();
                }
            } else {
                current_event.push(*c);
            }
        }

        offset += read_bytes;
    }

    if !current_event.is_empty() {
        let event: ChatEvent = serde_json::from_slice(current_event.as_slice()).unwrap();
        match event.kind {
            ChatEventKind::Connected() => {
                connected_users.insert(event.user);
            }
            ChatEventKind::Disconnected() => {
                connected_users.remove(&event.user);
            }
            _ => {}
        };
    }

    connected_users.into_iter().collect()
}

pub async fn get_client_address(ctx: Context, client: String) -> Option<Address> {
    let file_name = get_client_address_file_name(client);
    let mut file = ctx
        .open_file(&file_name)
        .await
        .map_err(|e| match e {
            StorageError::NotFound => e,
            _ => panic!("storage unavailable"),
        })
        .ok()?;
    let mut buf = vec![0u8; 4096];
    let bytes = file.read(0, &mut buf).await.unwrap();
    assert!(bytes > 0);
    Some(serde_json::from_slice(&buf.as_slice()[..bytes as usize]).unwrap())
}

pub async fn chat_exists(ctx: Context, chat: String) -> bool {
    ctx.file_exists(&get_chat_history_file_name(chat))
        .await
        .unwrap()
}

pub fn get_chat_history_file_name(chat: String) -> String {
    format!("{}.chat_history", chat)
}

pub fn get_client_history_file_name(client: String) -> String {
    format!("{}.client_history", client)
}

pub fn get_client_password_file_name(client: String) -> String {
    format!("{}.password", client)
}

pub fn get_global_requests_file_name() -> String {
    "requests.global".to_string()
}

pub fn get_client_address_file_name(client: String) -> String {
    format!("{}.address", client)
}
