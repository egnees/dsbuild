use std::thread;

use dsbuild::{Address, Context, Message, Process};

struct EchoServer {}

impl Process for EchoServer {
    fn on_local_message(&mut self, _msg: Message, _ctx: Context) {
        unreachable!()
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) {
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(msg, from, 5.0).await;
        });
    }
}

struct EchoClient {
    server: Address,
}

impl Process for EchoClient {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        let dst = self.server.clone();
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(msg, dst, 5.0).await;
        });
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, _from: Address, ctx: Context) {
        ctx.send_local(msg);
    }
}

#[test]
fn message_returns_virtual() {
    let mut sys = dsbuild::Sim::new(123);

    // configure network
    sys.set_network_delays(0.5, 1.5);
    sys.set_network_drop_rate(0.05);

    // add echo server
    sys.add_node("echo_server", "echo.server.ru", 80);
    sys.add_process("p", EchoServer {}, "echo_server");

    // add echo client
    sys.add_node("echo_client", "echo.client.ru", 80);
    sys.add_process(
        "p",
        EchoClient {
            server: Address::new_ref("echo.server.ru", 80, "p"),
        },
        "echo_client",
    );

    // local message from user
    sys.send_local_message("p", "echo_client", "ping".into());
    sys.step_until_no_events();

    // get returned message from server
    let msgs = sys.read_local_messages("p", "echo_client").unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].data::<String>().unwrap(), "ping");
}

#[test]
fn message_returns_real() {
    // create echo server node on host 127.0.0.1:10024
    let mut server = dsbuild::RealNode::new("127.0.0.1", 10024, "/tmp");
    let mut server_io = server.add_process(EchoServer {}, "p".into());

    // create echo client node on host 127.0.0.1:10025
    let mut client = dsbuild::RealNode::new("127.0.0.1", 10025, "/tmp");
    let mut client_io = client.add_process(
        EchoClient {
            server: Address::new_ref("127.0.0.1", 10024, "p"),
        },
        "p".into(),
    );

    // spawn async user activity
    client.spawn(async move {
        // send request
        let msg = "ping";
        println!("INFO sending message to server: {}", msg);
        client_io.sender.send(msg.into()).await.unwrap();

        // wait for response
        let msg = client_io
            .receiver
            .recv()
            .await
            .unwrap()
            .data::<String>()
            .unwrap();
        println!("INFO received message from server: {}", msg);
        assert_eq!(msg, "ping");

        // stop client and server
        client_io.stop_process().await;
        server_io.stop_process().await;
    });

    // run server in background
    let server_handle = thread::spawn(move || {
        server.run();
    });

    // run client in background
    let client_handle = thread::spawn(move || {
        client.run();
    });

    // wait for client and server complete
    client_handle.join().unwrap();
    server_handle.join().unwrap();
}

#[test]
fn server_fault_virtual() {
    let mut sys = dsbuild::Sim::new(321);

    sys.set_network_delays(0.5, 1.5);
    sys.set_network_drop_rate(0.05);

    sys.add_node("echo_server", "echo.server.ru", 80);
    sys.connect_node_to_network("echo_server");
    sys.add_process("p", EchoServer {}, "echo_server");

    sys.add_node("echo_client", "echo.client.ru", 80);
    sys.connect_node_to_network("echo_client");
    sys.add_process(
        "p",
        EchoClient {
            server: Address::new_ref("echo.server.ru", 80, "p"),
        },
        "echo_client",
    );

    sys.send_local_message("p", "echo_client", "first ping".into());
    sys.step_until_no_events();
    let msgs = sys.read_local_messages("p", "echo_client").unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].data::<String>().unwrap(), "first ping");

    // crash server
    sys.crash_node("echo_server");
    sys.step_until_no_events();

    sys.send_local_message("p", "echo_client", "second ping".into());
    sys.step_until_no_events();
    assert!(sys.read_local_messages("p", "echo_client").is_none());
}
