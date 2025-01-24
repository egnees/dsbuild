use std::{net::SocketAddr, time::Duration};

use dsbuild::Message;
use log::info;
use raft::local::{InitializeRequest, InitializeResponse, LocalResponse, LOCAL_RESPONSE};
use tokio::sync::mpsc;

use crate::{
    http::listener,
    register::{RequestRegister, SharedRequestRegister},
};

//////////////////////////////////////////////////////////////////////////////////////////

/// Run I/O for process
pub async fn process_io(
    node_id: usize,
    addr: SocketAddr,
    sender: mpsc::Sender<Message>,
    mut receiver: mpsc::Receiver<Message>,
    nodes: Vec<SocketAddr>,
) {
    // send initialize request
    sender
        .send(InitializeRequest {}.into())
        .await
        .expect("can not send initialize request");

    // receive initialize response
    let response: InitializeResponse = receiver
        .recv()
        .await
        .expect("can not receive initialize response")
        .into();

    info!("Initialized with seq_num={:?}", response.seq_num);

    let seq_num = response.seq_num;
    let register = RequestRegister::new(seq_num, nodes, node_id, sender);
    let shared_register = SharedRequestRegister::new(register);

    // spawn http listener cycle
    tokio::task::spawn({
        let register = shared_register.clone();
        async move {
            loop {
                let listen_result = listener(addr, register.clone()).await;
                if let Err(err) = listen_result {
                    eprintln!("Listen error: {:?}", err);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // run listener for process local messages
    loop {
        let message = receiver.recv().await;
        if let Some(message) = message {
            if message.tip() == LOCAL_RESPONSE {
                let local_response: LocalResponse = message.into();
                shared_register.respond(local_response).await;
            }
        } else {
            // sender dropped
            break;
        }
    }
}
