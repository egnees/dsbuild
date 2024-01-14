//! Pinger executor.

use dsbuild::{examples::ping_pong::pinger, Address, RealSystemConfig, RealSystem};

/// Accepts arguments from the command line.
/// * listen_host
/// * listen_port
/// * ponger_host
/// * ponger_port
fn main() {
    // Init logging.
    env_logger::Builder::new().filter_level(log::LevelFilter::Warn).init();

    // Parse command line arguments.
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 5 {
        println!("Usage: {} <listen_host> <listen_port> <ponger_host> <ponger_port>", args[0]);
        return;
    }

    // Get the listen_host.
    let listen_host = &args[1];

    // Get the listen_port.
    let listen_port = args[2].parse::<u16>().expect("Can not parse listen port");

    // Get ponger host and port.
    let ponger_host = &args[3];
    let ponger_port = args[4].parse::<u16>().expect("Can not parse ponger port");

    // Set ponger address.
    let ponger_addr = Address::new(ponger_host.to_string(), ponger_port, "PONGER".to_string());
    let pinger = pinger::create_pinger(1.0, ponger_addr, 100);

    // Create config.
    let config = RealSystemConfig::default(listen_host.to_owned(), listen_port).unwrap();

    // Create system with specified config.
    let mut system = RealSystem::new(config).unwrap();

    // Add pinger process to the system and get reference to it.
    let pinger_wrapper = system.add_process("PINGER", pinger).unwrap();

    // Run system.
    system.run().unwrap();

    // Check pinger is stopped.
    assert!(pinger_wrapper.read().is_stopped());
}