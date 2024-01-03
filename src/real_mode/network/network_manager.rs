use std::{marker::PhantomData, sync::Mutex};

use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{common::message::Message, real_mode::events::Event};

use super::{
    defs::{Address, ProcessSendRequest},
    grpc_messenger::GRpcMessenger,
    messenger::AsyncMessenger,
};

#[derive(Default)]
struct NetworkManager<M: AsyncMessenger> {
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
            return Err("Already listening".to_owned());
        }

        let handler = tokio::spawn(async move {
            M::listen(host, port, sender)
                .await
                .expect("Can not start listening")
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

static MANAGER: Mutex<Option<NetworkManager<GRpcMessenger>>> = Mutex::new(Option::None);

pub fn init() {
    let mut guard = MANAGER.lock().expect("Can not lock the network manager");

    *guard = Some(NetworkManager::default());
}

pub fn start_listen(host: String, port: u16, sender: Sender<Event>) -> Result<(), String> {
    let mut guard = MANAGER.lock().expect("Can not lock the network manager");
    let network_manager_ref = guard.as_mut().expect("Network manager is not initialized");

    network_manager_ref.start_listen(host, port, sender)
}

pub fn send_message(from: Address, to: Address, msg: Message) {
    let mut guard = MANAGER.lock().expect("Can not lock the network manager");
    let network_manager_ref = guard.as_mut().expect("Network manager is not initialized");

    network_manager_ref.send_message(from, to, msg)
}

pub fn stop_listen() {
    let mut guard = MANAGER.lock().expect("Can not lock the network manager");
    let network_manager_ref = guard.as_mut().expect("Network manager is not initialized");

    network_manager_ref.stop_listen()
}
