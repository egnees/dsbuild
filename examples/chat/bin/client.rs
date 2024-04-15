use chat::{start_io, Client};
use dsbuild::{Address, RealSystem};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 6 {
        println!(
            "Usage: {} <server_ip> <server_port> <client_ip> <client_port> <name>",
            args[0]
        );
        return;
    }

    let server_ip = &args[1];
    let server_port = args[2].parse::<u16>().expect("Can not parse server port");

    let client_ip = &args[3];
    let client_port = args[4].parse::<u16>().expect("Can not parse listen port");
    let client_name = &args[5];

    let server_address = Address {
        host: server_ip.to_owned(),
        port: server_port,
        process_name: "chat_server".into(),
    };

    let self_address = Address {
        host: client_ip.to_owned(),
        port: client_port,
        process_name: client_name.clone(),
    };

    let mut system = RealSystem::new(1024, client_ip, client_port, "storage_path".into());

    let io = system.add_process(
        Client::new(
            server_address,
            self_address,
            client_name.clone(),
            "pass123".into(),
        ),
        client_name.to_owned(),
    );

    system.spawn(start_io(io));

    system.run();
}
