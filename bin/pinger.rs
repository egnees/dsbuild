use std::env;

use dsbuild::{Address, AddressResolvePolicy, Message, Process, RealSystem, RealSystemConfig};

#[derive(Clone)]
struct PingProcess {
    last_pong: u32,
    need_pongs: u32,
    retry_delay: f64,
    ponger: String,
    received_all: bool,
}

impl PingProcess {
    fn ping(&mut self, ctx: &mut dyn dsbuild::Context) -> Result<(), String> {
        println!(
            "Ping process {} with pong request {}",
            self.ponger,
            self.last_pong + 1
        );

        let msg = Message::borrow_new("PING", self.last_pong + 1)?;
        ctx.send_message(msg, self.ponger.clone());
        ctx.set_timer("PING_TIMER".to_owned(), self.retry_delay);

        Ok(())
    }

    fn new(ponger: String, need_pings: u32, retry_delay: f64) -> Self {
        assert!(need_pings > 0);

        Self {
            last_pong: 0,
            need_pongs: need_pings,
            retry_delay,
            ponger,
            received_all: false,
        }
    }
}

impl Process for PingProcess {
    fn on_start(&mut self, ctx: &mut dyn dsbuild::Context) -> Result<(), String> {
        println!("Starting ping process...");

        self.ping(ctx)
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn dsbuild::Context) -> Result<(), String> {
        assert_eq!(name, "PING_TIMER".to_owned());

        self.ping(ctx)
    }

    fn on_message(
        &mut self,
        msg: dsbuild::Message,
        from: String,
        ctx: &mut dyn dsbuild::Context,
    ) -> Result<(), String> {
        assert_eq!(msg.get_tip(), "PONG");

        assert_eq!(from, self.ponger);

        let pong = msg.get_data::<u32>()?;
        if pong == self.last_pong + 1 {
            self.last_pong += 1;
        }

        assert!(self.last_pong <= self.need_pongs);

        if self.last_pong < self.need_pongs {
            self.ping(ctx)
        } else {
            println!("Stopping ping process...");

            self.received_all = true;

            ctx.stop_process();

            Ok(())
        }
    }
}

fn main() {
    // Get arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!(
            "Usage: {} <listen_port> <ponger_host> <ponger_port> <ping_count>",
            args[0]
        );
        return;
    }

    // Get own address and ponger address.
    let listen_port = args[1].parse::<u16>().expect("Can not parse listen port");
    let ponger_host = args[2].clone();
    let ponger_port = args[3].parse::<u16>().expect("Can not parse send port");

    // Define ponger address.
    let ponger_address = Address {
        host: ponger_host,
        port: ponger_port,
        process_name: "ponger".to_owned(),
    };

    // Create system.
    let resolve_policy = AddressResolvePolicy::Manual {
        resolve_list: vec![ponger_address],
    };
    let config = RealSystemConfig::new_with_max_threads(
        8,
        resolve_policy,
        "127.0.0.1".to_owned(),
        listen_port,
    )
    .expect("Can not create system config");
    let mut system = RealSystem::new(config).expect("Can not create system");

    // Create ping process.
    let ping_count = args[4].parse::<u32>().expect("Can not parse ping count");
    let ping_process = PingProcess::new("ponger".to_owned(), ping_count, 0.2);

    // Add ping process to the system.
    let wrapper = system
        .add_process("pinger", ping_process)
        .expect("Can not add ping process");

    // Run system.
    system.run().expect("Can not run system");

    // Check if all pongs were received.
    assert!(wrapper.read().received_all);
}
