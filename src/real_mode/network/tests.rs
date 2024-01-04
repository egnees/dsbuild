use std::sync::{Arc, Mutex};

use tokio::{runtime::Runtime, sync::mpsc};

use crate::{
    common::message::Message,
    real_mode::{events::Event, network::resolver::AddressResolver},
};

use super::{
    defs::*, grpc_messenger::GRpcMessenger, manual_resolver::ManualResolver,
    messenger::AsyncMessenger, network_manager::NetworkManager,
};

#[test]
fn test_grpc_messenger() {
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
        GRpcMessenger::listen(
            ponger_address.host.clone(),
            ponger_address.port,
            ping_send_to,
        )
        .await
        .expect("Can not listen");
    });

    // Create channel in which network listener of pong messages must pass accepted pongs
    let (pong_send_to, mut pong_receiver) = mpsc::channel(1024);

    // Spawn pinger which waits for the pong messages
    let pinger_address = ping_address.clone();
    runtime.spawn(async move {
        GRpcMessenger::listen(
            pinger_address.host.clone(),
            pinger_address.port,
            pong_send_to,
        )
        .await
        .expect("Can not start listen ping messages")
    });

    // Create true ping message and ping request which can be compared with ping request received by ponger
    let ping_message =
        Message::new("PING", &"ping_msg".to_string()).expect("Can not create ping message");

    // Ping request and associated ping message event
    let ping_request = ProcessSendRequest {
        sender_address: ping_address.clone(),
        receiver_address: pong_address.clone(),
        message: ping_message.clone(),
    };
    let ping_message_event = Event::MessageReceived {
        msg: ping_message.clone(),
        from: ping_address.process_name.clone(),
        to: pong_address.process_name.clone(),
    };

    // Create true pong message and pong request which can be compared with ping request received by ponger
    let pong_message =
        Message::new("PONG", &"pong_msg".to_string()).expect("Can not create pong message");

    // Pong request which is response on ping and associated pong message event
    let pong_message_request = ProcessSendRequest {
        sender_address: pong_address.clone(),
        receiver_address: ping_address.clone(),
        message: pong_message.clone(),
    };
    let pong_message_event = Event::MessageReceived {
        msg: pong_message.clone(),
        from: pong_address.process_name,
        to: ping_address.process_name,
    };

    // Clone requests to pass them into closure
    let send_ping_request = ping_request.clone();
    let send_pong_request = pong_message_request.clone();

    // Ensure that pong request is received
    let got_pong_request = Arc::new(Mutex::new(bool::from(false)));
    let got_pong_request2 = got_pong_request.clone();

    // Create pinger process which sends one ping to ponger process
    // and waits for the pong from him
    let pinger = async move {
        // Send request to ponger
        GRpcMessenger::send(send_ping_request)
            .await
            .expect("Can not join await for send ping message");

        // Wait for answer from the recv channel
        let received_pong_response = pong_receiver
            .recv()
            .await
            .expect("Can not receive pong request");

        // Check that received pong response is equal to the pong request
        // which was send by ponger
        assert_eq!(received_pong_response, pong_message_event);

        // Mark pong request as received
        *got_pong_request2.lock().expect("Can not lock mutex") = true;
    };

    // Create ponger process which waits for the ping from pinger process
    // and then sends pong to him
    let ponger = async move {
        // Receive ping request
        let received_ping_event = ping_receiver
            .recv()
            .await
            .expect("Can not get ping request");

        // Check what received ping request is equal to the
        // true ping request, which was send by pinger
        assert_eq!(received_ping_event, ping_message_event);

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

#[test]
fn test_manual_resolver() {
    // Create resolver.
    let mut resolver = ManualResolver::default();

    // Add the first process address.
    let first_address = Address {
        host: "12345".to_owned(),
        port: 12345,
        process_name: "process1".to_owned(),
    };

    resolver
        .add_record(&first_address)
        .expect("Can not add address with process name which is not present");

    // Add the other one first process address.
    let new_first_address = Address {
        host: "123".to_owned(),
        port: 123,
        process_name: "process1".to_owned(),
    };

    resolver
        .add_record(&new_first_address)
        .expect_err("Resolver allows to add address with the same process name twice");

    // Try to resolve the first one address by the first process name.
    assert_eq!(
        first_address,
        resolver
            .resolve("process1")
            .expect("Can not resolve the first process address")
    );

    // Try to resolve address by not existing process name.
    resolver
        .resolve("process3")
        .expect_err("Resolver allows to resolve address by not registered process name");

    // Check the second address can be added.
    let second_address = Address {
        host: "12345".to_owned(),
        port: 1223,
        process_name: "process2".to_owned(),
    };

    resolver
        .add_record(&second_address)
        .expect("Can not add the second process address");

    // Check the second process address can be resolved.
    assert_eq!(
        second_address,
        resolver
            .resolve("process2")
            .expect("Can not resolve the second process address")
    );

    // Check that the first one address still can be resolved.
    assert_eq!(
        first_address,
        resolver
            .resolve("process1")
            .expect("Can not resolve the first process address in the second time")
    );
}

#[test]
fn test_network_manager() {
    // Initialize listener address.
    let listen_address = Address {
        host: "127.0.0.1".to_owned(),
        port: 59938,
        process_name: "listener".to_owned(),
    };

    // Initialize sender address.
    let send_address = Address {
        host: "127.0.0.1".to_owned(),
        port: 59939,
        process_name: "sender".to_owned(),
    };

    // Create runtime.
    let runtime = tokio::runtime::Runtime::new().expect("Can not create runtime");

    // Initialzie network manager.
    let network_manager = Arc::new(Mutex::new(NetworkManager::<GRpcMessenger>::default()));

    let (sender, mut receiver) = mpsc::channel(32);

    // Spawn listener.
    let listen_addr = listen_address.clone();
    // Create listen network manager clone.
    let listen_network_manager_clone = network_manager.clone();
    runtime.spawn(async move {
        // Start listen.
        listen_network_manager_clone
            .lock()
            .expect("Can not lock network manager")
            .start_listen(listen_addr.host, listen_addr.port, sender)
            .expect("Can not start listen");
    });

    // Spawn sender.
    let sender_addr = send_address.clone();
    let listen_addr = listen_address.clone();
    let message =
        Message::borrow_new("hello_message", "hello".to_owned()).expect("Can not create message");
    let message_clone = message.clone();
    runtime.spawn(async move {
        // Send message.
        network_manager
            .lock()
            .expect("Can not lock network manager")
            .send_message(sender_addr, listen_addr, message_clone);
    });

    // Wait for message.
    let sender_addr = send_address.clone();
    let listen_addr = listen_address.clone();
    runtime.block_on(async move {
        // Wait for message.
        let received_message_event = receiver.recv().await.expect("Can not receive message");

        // Check that received message is equal to the sent message.
        match received_message_event {
            Event::TimerFired {
                process_name: _,
                timer_name: _,
            } => panic!("Incorrect event received"),
            Event::MessageReceived { msg, from, to } => {
                assert_eq!(msg, message);
                assert_eq!(from, sender_addr.process_name);
                assert_eq!(to, listen_addr.process_name);
            }
            Event::SystemStarted {} => panic!("Incorrect event received"),
        }
    });
}
