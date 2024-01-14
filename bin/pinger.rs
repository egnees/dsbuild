//! Pinger executor.

use dsbuild::{examples::ping_pong::pinger, Address, RealSystemConfig, RealSystem};

/// Accepts arguments from the command line.
/// * listen_port
/// * ponger_host
/// * ponger_port
fn main() {
    // Parse command line arguments.
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 4 {
        println!("Usage: {} <listen_port> <ponger_host> <ponger_port>", args[0]);
        return;
    }

    // Get the listen_port.
    let listen_port = args[1].parse::<u16>().expect("Can not parse listen port");

    // Get ponger host and port.
    let ponger_host = &args[2];
    let ponger_port = args[3].parse::<u16>().expect("Can not parse ponger port");

    // Set ponger address.
    let ponger_addr = Address::new(ponger_host.to_string(), ponger_port, "PONGER".to_string());
    let pinger = pinger::create_pinger(0.1, ponger_addr, 100);

    // Create config.
    let config = RealSystemConfig::default("127.0.0.1".to_owned(), listen_port).unwrap();

    // Create system with specified config.
    let mut system = RealSystem::new(config).unwrap();

    // Add pinger process to the system and get reference to it.
    let pinger_wrapper = system.add_process("PINGER", pinger).unwrap();

    // Run system.
    system.run().unwrap();

    // Check pinger is stopped.
    assert!(pinger_wrapper.read().is_stopped());
}