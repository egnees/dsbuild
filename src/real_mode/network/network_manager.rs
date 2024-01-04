use std::marker::PhantomData;

use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{common::message::Message, real_mode::events::Event};

use super::{
    defs::{Address, ProcessSendRequest},
    messenger::AsyncMessenger,
};

#[derive(Default)]
pub struct NetworkManager<M: AsyncMessenger> {
    _phantom: PhantomData<M>,
    listen_handler: Option<JoinHandle<()>>,
}

impl<M: AsyncMessenger> NetworkManager<M> {
    pub fn start_listen(
        &mut self,
        host: String,
        port: u16,
        sender: Sender<Event>,
    ) -> Result<(), String> {
        if self.listen_handler.is_some() {
            return Err("Can not start listen: already listening".to_owned());
        }

        let handler = tokio::spawn(async move {
            M::listen(host, port, sender)
                .await
                .map_err(|e| "Can not start listen: ".to_owned() + e.to_string().as_str())
                .unwrap();
        });

        self.listen_handler = Some(handler);

        Ok(())
    }

    pub fn send_message(&mut self, from: Address, to: Address, msg: Message) {
        let request = ProcessSendRequest {
            sender_address: from,
            receiver_address: to,
            message: msg,
        };

        tokio::spawn(async move { M::send(request).await.expect("Can not send message") });
    }

    pub fn stop_listen(&mut self) {
        if let Some(handler) = &mut self.listen_handler {
            handler.abort();
        }

        self.listen_handler = None;
    }
}
