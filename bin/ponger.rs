//! Ponger executor.

use dsbuild::{examples::ping_pong::ponger, RealSystemConfig, RealSystem};

/// Accepts arguments from the command line.
/// * listen_host
/// * listen_port
fn main() {
    // Init logging.
    env_logger::Builder::new().filter_level(log::LevelFilter::Warn).init();

    // Parse command line arguments.
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("Usage: {} <listen_host> <listen_port>", args[0]);
        return;
    }

    // Get the listen_host.
    let listen_host = &args[1];

    // Get the listen_port.
    let listen_port = args[2].parse::<u16>().expect("Can not parse listen port");

    // Create ponger process.
    let ponger = ponger::create_ponger(60.0 * 60.0); // 60 minutes.

    // Create system config.
    let config = RealSystemConfig::default(listen_host.to_owned(), listen_port).unwrap();

    // Create system with specified config.
    let mut system = RealSystem::new(config).unwrap();

    // Add ponger process to the system and get reference to it.
    let ponger_wrapper = system.add_process("PONGER", ponger).unwrap();

    // Run system.
    system.run().unwrap();

    // Check ponger is stopped.
    assert!(ponger_wrapper.read().is_stopped());
}