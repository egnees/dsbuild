use std::env;

use dsbuild::common::context::Context;
use dsbuild::common::message::Message;
use dsbuild::common::process::Process;
use dsbuild::real_mode::system::*;

#[derive(Clone)]
struct PongProcess {
    // window of inactivity after that pong process will be stopped
    delay: f64,
    // stopped flag
    stopped: bool,
}

impl PongProcess {
    const TIMER_NAME: &'static str = "PONG_TIMER";

    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.delay);
    }

    pub fn new(delay: f64) -> Self {
        Self {
            delay,
            stopped: false,
        }
    }
}

impl Process for PongProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        println!("Starting pong process...");

        self.set_timer(ctx);

        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);

        println!("Stopping pong process...");

        self.stopped = true;
        ctx.stop_process();
        Ok(())
    }

    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String> {
        let requested_pong = msg.get_data::<u32>()?;
        let answer = Message::borrow_new("PONG", requested_pong)?;
        ctx.send_message(answer, from);
        self.set_timer(ctx);

        println!("Answer pong request: {}", requested_pong);

        Ok(())
    }
}

fn main() {
    // Get arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!(
            "Usage: {} <listen_port> <pinger_host> <pinger_port>",
            args[0]
        );
        return;
    }

    // Get own address and pinger address.
    let listen_port = args[1].parse::<u16>().expect("Can not parse listen port");
    let pinger_host = args[2].clone();
    let pinger_port = args[3].parse::<u16>().expect("Can not parse send port");

    // Define pinger address.
    let pinger_address = Address {
        host: pinger_host,
        port: pinger_port,
        process_name: "pinger".to_owned(),
    };

    // Create system.
    let resolve_policy = AddressResolvePolicy::Manual {
        resolve_list: vec![pinger_address],
    };

    let config =
        SystemConfig::new_with_max_threads(8, resolve_policy, "127.0.0.1".to_owned(), listen_port)
            .expect("Can not create system config");

    let mut system = System::new(config).expect("Can not create system");

    // Create ping process.
    let pong_process = PongProcess::new(0.5);

    // Add ping process to the system.
    let wrapper = system
        .add_process("ponger", pong_process)
        .expect("Can not add pong process");

    // Run system.
    system.run().expect("Can not run system");

    // Check if all pongs were received.
    assert!(wrapper.read().stopped);
}
