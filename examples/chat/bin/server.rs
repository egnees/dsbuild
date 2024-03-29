use chat::Server;
use dsbuild::RealSystem;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        println!("Usage: {} <listen_port>", args[0]);
        return;
    }

    let listen_port = args[1].parse::<u16>().expect("Can not parse listen port");

    let mut system = RealSystem::new(1024, "127.0.0.1", listen_port);

    system.add_process(Server::new("SERVER".into()), "chat_server".into());

    system.run();
}
