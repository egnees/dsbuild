use chat::Server;
use dsbuild::RealSystem;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("Usage: {} <listen_host> <listen_port>", args[0]);
        return;
    }

    let listen_host = &args[1];
    let listen_port = args[2].parse::<u16>().expect("Can not parse listen port");

    let mut system = RealSystem::new(1024, listen_host, listen_port);

    system.add_process(Server::new("SERVER".into()), "chat_server".into());

    system.run();
}
