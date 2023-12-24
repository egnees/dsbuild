use std::sync::{Arc, Mutex};

use tokio::{runtime::Runtime, sync::mpsc};

use crate::common::message::Message;

use super::{
    grpc_messenger::GRpcMessenger,
    messenger::{Address, AsyncMessenger, ProcessSendRequest},
};

#[test]
fn test_grpc_manager() {
    // Create runtime
    let runtime = Runtime::new().expect("Can not create tokio runtime");

    // Address of process which pings the other one
    let ping_address = Address {
        host: "127.0.0.1".to_owned(),
        port: 50995,
        process_name: "ping_process".to_owned(),
    };

    // Address of process which answers with pong
    let pong_address = Address {
        host: "127.0.0.1".to_owned(),
        port: 50996,
        process_name: "pong_process".to_owned(),
    };

    // Create channel in which network listener of ping messages must pass accepted pings
    let (ping_send_to, mut ping_receiver) = mpsc::channel(1024);

    // Spawn ponger task which listens to pong messages
    let ponger_address = pong_address.clone();
    runtime.spawn(async move {
        GRpcMessenger::listen(&ponger_address.host, ponger_address.port, ping_send_to)
            .await
            .expect("Can not listen");
    });

    // Create channel in which network listener of pong messages must pass accepted pongs
    let (pong_send_to, mut pong_receiver) = mpsc::channel(1024);

    // Spawn pinger which waits for the pong messages
    let pinger_address = ping_address.clone();
    runtime.spawn(async move {
        GRpcMessenger::listen(&pinger_address.host, pinger_address.port, pong_send_to)
            .await
            .expect("Can not start listen ping messages")
    });

    // Create true ping message and ping request which can be compared with ping request received by ponger
    let ping_message =
        Message::new("PING", &"ping_msg".to_string()).expect("Can not create ping message");

    // Ping request
    let ping_request = ProcessSendRequest {
        sender_address: ping_address.clone(),
        receiver_address: pong_address.clone(),
        message: ping_message.clone(),
    };

    // Create true pong message and pong request which can be compared with ping request received by ponger
    let pong_message =
        Message::new("PONG", &"pong_msg".to_string()).expect("Can not create pong message");

    // Pong request which is response on ping
    let pong_request = ProcessSendRequest {
        sender_address: pong_address.clone(),
        receiver_address: ping_address.clone(),
        message: pong_message.clone(),
    };

    // Clone requests to pass them into closure
    let send_ping_request = ping_request.clone();
    let send_pong_request = pong_request.clone();

    // Ensure that pong request is received
    let got_pong_request = Arc::new(Mutex::new(bool::from(false)));
    let got_pong_request2 = got_pong_request.clone();

    // Create pinger process which sends one ping to ponger process
    // and waits for the pong from him
    let pinger = async move {
        // Send request to ponger
        GRpcMessenger::send(send_ping_request)
            .await
            .expect("Can join await for send ping message");

        // Wait for answer from the recv channel
        let received_pong_response = pong_receiver
            .recv()
            .await
            .expect("Can not receive pong request");

        // Check that received pong response is equal to the pong request
        // which was send by ponger
        assert_eq!(received_pong_response, pong_request);

        // Mark pong request as received
        *got_pong_request2.lock().expect("Can not lock mutex") = true;
    };

    // Create ponger process which waits for the ping from pinger process
    // and then sends pong to him
    let ponger = async move {
        // Receive ping request
        let received_ping_request = ping_receiver
            .recv()
            .await
            .expect("Can not get ping request");

        // Check what received ping request is equal to the
        // true ping request, which was send by pinger
        assert_eq!(received_ping_request, ping_request);

        // Send pong in response to the received ping
        GRpcMessenger::send(send_pong_request)
            .await
            .expect("Can not join await for send pong request");
    };

    // Spawn ponger
    runtime.spawn(ponger);

    // Block on pinger to wait for the
    // response from ponger
    runtime.block_on(pinger);

    // Check that pong request was received
    assert_eq!(*got_pong_request.lock().expect("Can not lock mutex"), true);
}
