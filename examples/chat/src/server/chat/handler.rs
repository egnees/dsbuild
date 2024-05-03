//! Definition of chat event appearance handler.

use dsbuild::{Address, Context};

use super::{
    event::{ChatEvent, ChatEventKind},
    manager::ChatGuard,
};

/// Responsible for handling new chat event from user and sending responses to it.
/// Owns [`chat guard`][`ChatGuard`], which guarantees exclusive access to the chat on the handle period.
/// Updates list of connected users if user connected or disconnected.
/// Caller must guarantee that producer is not connected to a chat in cases of
/// [`ChatEventKind::Connected`] and [`ChatEventKind::Created`],
/// or that producer is connected to the chat in all other cases.
pub struct RequestHandler<'a> {
    /// Guard of initialized history.
    pub chat_guard: ChatGuard<'a>,
    /// Chat event which should be processed.
    pub event_kind: ChatEventKind,
    /// User, who produced event.
    pub producer: String,
    /// Address of producer.
    pub address: Address,
    /// [`dsbuild::Context`].
    pub ctx: Context,
}

impl<'a> RequestHandler<'a> {
    pub fn new(
        chat_guard: ChatGuard<'a>,
        event: ChatEventKind,
        producer: String,
        address: Address,
        ctx: Context,
    ) -> Self {
        Self {
            chat_guard,
            event_kind: event,
            producer,
            address,
            ctx,
        }
    }

    /// Handle the request.
    /// One-shot.
    pub async fn handle(mut self) -> Option<(ChatEvent, Vec<String>)> {
        assert!(self.chat_guard.is_initialized);

        let new_connection = match &self.event_kind {
            ChatEventKind::Connected() => true,
            _ => false,
        };

        let lost_connection = match &self.event_kind {
            ChatEventKind::Disconnected() => true,
            _ => false,
        };

        let trying_to_create = match &self.event_kind {
            ChatEventKind::Created() => true,
            _ => false,
        };

        if trying_to_create ^ (self.chat_guard.history_size == 0) {
            return None;
        }

        let chat_event = ChatEvent::new_with_kind(
            self.chat_guard.chat_name.clone(),
            self.producer.clone(),
            self.event_kind,
            self.chat_guard.history_size,
        );

        self.chat_guard
            .extend_history(chat_event.clone(), self.ctx.clone())
            .await;

        if new_connection {
            self.chat_guard.connected_users.insert(self.producer);

            self.chat_guard
                .transfer_history_by_address(self.address, self.ctx)
                .await;
        } else if lost_connection {
            self.chat_guard.connected_users.remove(&self.producer);
        }

        Some((
            chat_event,
            self.chat_guard
                .connected_users
                .clone()
                .into_iter()
                .collect(),
        ))
    }
}
