use chat::{start_io, Client};
use dsbuild::{Address, RealSystem};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("Usage: {} <port> <name>", args[0]);
        return;
    }

    let port = args[1].parse::<u16>().expect("Can not parse listen port");

    if port == 11085 {
        println!("Can not start of server port: {}", port);
        return;
    }

    let name = args[2].clone();

    let server_address = Address {
        host: "127.0.0.1".into(),
        port: 11085,
        process_name: "chat_server".into(),
    };

    let self_address = Address {
        host: "127.0.0.1".into(),
        port,
        process_name: name.clone(),
    };

    let mut system = RealSystem::new(1024, "127.0.0.1", port);

    let io = system.add_process(
        Client::new(server_address, self_address, name.clone(), "pass123".into()),
        name.clone().into(),
    );

    system.spawn(start_io(io));

    system.run();
}
