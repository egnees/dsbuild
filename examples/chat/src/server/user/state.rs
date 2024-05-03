/// Definition of user state.
use dsbuild::{Address, Context};

use crate::server::chat::event::ChatEventKind;

use super::handler::RelatedChatEventKind;

/// Responsible for all user addresses and connections.
pub struct UserState {
    /// List of user addresses.
    pub addr: Address,
    pub connected_chat: Option<String>,
    pub password: Option<String>,
    pub is_initialized: bool,
    file_name: String,
}

impl UserState {
    /// Creates a new user state.
    pub fn new(user_name: String, addr: Address) -> Self {
        Self {
            addr,
            connected_chat: None,
            password: None,
            is_initialized: false,
            file_name: user_name + ".user",
        }
    }

    /// Initializer user state with even provided password, if there is no record about user on disk,
    /// or compares the password on disk with provided password and returns true if they are the same.
    pub async fn init(&mut self, ctx: Context, password: &str, addr: &Address) -> bool {
        if !ctx.file_exists(&self.file_name).await.unwrap() {
            ctx.create_file(&self.file_name).await.unwrap();
            ctx.append(&self.file_name, password.as_bytes())
                .await
                .unwrap();
            self.password = Some(password.to_owned());
            self.is_initialized = true;
            return true;
        }

        if !self.auth(ctx, password).await {
            return false;
        }

        self.addr = addr.clone();
        self.is_initialized = true;
        true
    }

    /// Update state with applied chat event.
    pub fn update(&mut self, event: RelatedChatEventKind) {
        match event.kind {
            ChatEventKind::SentMessage(_) => {}
            ChatEventKind::Connected() => self.connected_chat = Some(event.chat),
            ChatEventKind::Disconnected() => self.connected_chat = None,
            ChatEventKind::Created() => {}
        }
    }

    /// Make auth and returns true if case password is correct.
    async fn auth(&mut self, ctx: Context, password: &str) -> bool {
        if self.password.is_none() {
            let mut buf = vec![0; 4096];
            let mut real_password = String::new();
            let mut offset = 0;
            loop {
                let read_bytes = ctx.read(&self.file_name, offset, &mut buf).await.unwrap();
                if read_bytes == 0 {
                    break;
                }
                real_password.push_str(std::str::from_utf8(&buf[..read_bytes]).unwrap());
                offset += read_bytes;
            }

            self.password = Some(real_password);
        }

        password == self.password.as_ref().unwrap()
    }
}
