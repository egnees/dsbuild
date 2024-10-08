//! Definition of asynchronous network manager.

use log::{info, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::common::{
    message::RoutedMessage,
    network::{SendError, SendResult},
};

use super::messenger::{GRpcMessenger, ProcessSendRequest};

pub enum NetworkRequest {
    SendMessage(RoutedMessage),
    #[allow(dead_code)]
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
        tag: msg.tag,
    })
    .await;

    if let Err(info) = result {
        warn!(
            "Can not send message from {:?} to {:?};\n{}",
            msg.from, msg.to, info
        );
    }
}

pub async fn send_message_with_ack(msg: RoutedMessage) -> SendResult<()> {
    let result = GRpcMessenger::send(ProcessSendRequest {
        sender_address: msg.from.clone(),
        receiver_address: msg.to.clone(),
        message: msg.msg,
        tag: msg.tag,
    })
    .await;

    if let Ok(response) = result {
        if response.status == "success" {
            Ok(())
        } else {
            Err(SendError::NotSent)
        }
    } else {
        Err(SendError::NotSent)
    }
}

pub async fn send_message_with_ack_timeout(msg: RoutedMessage, timeout: f64) -> SendResult<()> {
    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_secs_f64(timeout)) => Err(SendError::Timeout),
        send_result = send_message_with_ack(msg) => send_result
    }
}
