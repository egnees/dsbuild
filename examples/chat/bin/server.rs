use chat::server::process::ServerProcess;
use dsbuild::RealSystem;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 4 {
        println!("Usage: {} <listen_host> <listen_port> <mount_dir>", args[0]);
        return;
    }

    let listen_host = &args[1];
    let listen_port = args[2].parse::<u16>().expect("Can not parse listen port");
    let mount_dir = &args[3];

    let mut system = RealSystem::new(1024, listen_host, listen_port, mount_dir.to_string());

    system.add_process(ServerProcess::default(), "server".to_owned());

    system.run();
}
