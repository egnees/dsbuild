use dsbuild::{Address, Context};

use super::{event::ChatEvent, messages::ServerMessage};

pub fn send_err(ctx: Context, request_id: u64, to: Address, info: String) {
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

pub async fn auth_user(ctx: Context, login: String, password: String) -> bool {
    let file_name = format!("{}.user", login);
    let user_registered = ctx.file_exists(&file_name).await.unwrap();
    if user_registered {
        let mut buf = vec![0; 4096];
        let mut real_password = String::new();
        let mut offset = 0;
        let mut file = ctx.open_file(&file_name).await.unwrap();
        loop {
            let read_bytes = file.read(offset, &mut buf).await.unwrap();
            if read_bytes == 0 {
                break;
            }
            real_password.push_str(std::str::from_utf8(&buf[..read_bytes as usize]).unwrap());
            offset += read_bytes;
        }
        real_password == password
    } else {
        let mut file = ctx.create_file(&file_name).await.unwrap();
        let data = password.as_bytes();
        let mut offset = 0;
        loop {
            let appended = file.append(&data[offset..]).await.unwrap();
            if appended == 0 {
                assert_eq!(offset, password.len());
                break;
            } else {
                offset += appended as usize;
            }
        }
        true
    }
}

pub async fn transfer_chat_history(ctx: Context, to: Address, chat: String) {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = format!("{}.chat", chat);
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
                        serde_json::from_slice(&current_event.as_slice()).unwrap();

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
        let event: ChatEvent = serde_json::from_slice(&current_event.as_slice()).unwrap();

        let msg = ServerMessage::ChatEvent(chat, event).into();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(msg, to, 5.0).await;
        });
    }
}

pub async fn calc_events_in_chat(ctx: Context, chat: String) -> u64 {
    let mut offset = 0;
    let mut buf = vec![0; 4096];
    let file_name = format!("{}.chat", chat);
    let mut file = ctx.open_file(&file_name).await.unwrap();

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
