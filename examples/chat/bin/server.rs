use chat::Server;
use dsbuild::RealSystem;

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut system = RealSystem::new(1024, "127.0.0.1", 11085);

    system.add_process(Server::new("SERVER".into()), "chat_server".into());

    system.run();
}
