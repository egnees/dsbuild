use dsbuild::{Address, Context, Message, Passable, Process};
use dsbuild_message::Tipped;
use serde::{Deserialize, Serialize};

// Define message types.
#[derive(Passable, Serialize, Deserialize)]
pub struct LocalPingRequest {
    pub receiver: Address,
}

#[derive(Passable, Serialize, Deserialize)]
pub struct Ping {}

#[derive(Passable, Serialize, Deserialize)]
pub struct Pong {}

// Define ping-pong process.
#[derive(Default)]
pub struct PingPongProcess {
    pub pings_received: usize,
    pub pongs_received: usize,
}

impl PingPongProcess {
    // Send ping message to the receiver.
    fn send_ping(&self, receiver: Address, ctx: Context) {
        let message = Message::new(Ping::TIP, &Ping {}).unwrap();
        ctx.send(message, receiver);
    }

    // Send pong message to the receiver.
    fn send_pong(&self, receiver: Address, ctx: Context) {
        let message = Message::new(Pong::TIP, &Pong {}).unwrap();
        ctx.send(message, receiver);
    }
}

impl Process for PingPongProcess {
    // Method will be called on receiving
    // local message from user.
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        assert_eq!(msg.get_tip(), LocalPingRequest::TIP);
        let request = msg.get_data::<LocalPingRequest>().unwrap();
        let receiver = request.receiver;
        self.send_ping(receiver, ctx);
    }

    // Method will be called on timer firing,
    // which is not relevant for the example.
    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    // Method will be called on received
    // netwrok message from other process.
    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) {
        ctx.send_local(msg.clone());
        match msg.get_tip().as_str() {
            Ping::TIP => {
                self.pings_received += 1;
                self.send_pong(from, ctx);
            }
            Pong::TIP => {
                self.pongs_received += 1;
            }
            _ => unreachable!(),
        }
    }
}
