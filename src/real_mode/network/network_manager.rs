use std::marker::PhantomData;

use log::warn;
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{
    common::{message::Message, process::Address},
    real_mode::events::Event,
};

use super::{defs::ProcessSendRequest, messenger::AsyncMessenger};

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
            let listen_result = M::listen(host.clone(), port, sender).await;

            if let Err(info) = listen_result {
                warn!("Can not start listen on {}:{};\n{}.", host, port, info);
            }
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

        tokio::spawn(async move {
            let send_result = M::send(request.clone()).await;
            if let Err(info) = send_result {
                warn!(
                    "Can not send message from {:?} to {:?};\n{}.",
                    &request.sender_address, &request.receiver_address, info
                );
            }
        });
    }

    pub fn stop_listen(&mut self) {
        if let Some(handler) = &mut self.listen_handler {
            handler.abort();
        }

        self.listen_handler = None;
    }
}
