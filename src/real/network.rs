//! Definition of async network manager.

use log::{info, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::common::message::RoutedMessage;

use super::messenger::{GRpcMessenger, ProcessSendRequest};

pub enum NetworkRequest {
    SendMessage(RoutedMessage),
    Suspend(),
}

pub async fn handle(
    msg_receiver: Sender<RoutedMessage>,
    mut listen_to: Receiver<NetworkRequest>,
    host: String,
    port: u16,
) {
    let listen_handler = tokio::spawn(async move {
        let listen_result = GRpcMessenger::listen(host.clone(), port, msg_receiver).await;

        if let Err(info) = listen_result {
            log::error!("Can not start listen on {}:{};\n{}.", host, port, info);
        }
    });

    tokio::spawn(async move {
        while let Some(request) = listen_to.recv().await {
            match request {
                NetworkRequest::SendMessage(routed_msg) => {
                    tokio::spawn(send_message(routed_msg));
                }
                NetworkRequest::Suspend() => {
                    break;
                }
            }
        }
        listen_handler.abort();

        info!("Suspended network listening");
    });
}

async fn send_message(msg: RoutedMessage) {
    let result = GRpcMessenger::send(ProcessSendRequest {
        sender_address: msg.from.clone(),
        receiver_address: msg.to.clone(),
        message: msg.msg,
    })
    .await;

    if let Err(info) = result {
        warn!(
            "Can not send message from {:?} to {:?};\n{}",
            msg.from, msg.to, info
        );
    }
}
