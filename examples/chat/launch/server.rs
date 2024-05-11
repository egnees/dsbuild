use chat::{server::process::ServerProcess, utils::server::check_replica_request};
use dsbuild::{Address, RealSystem};

use crate::cfg::ServerConfig;

pub fn run_server_with_config(config: ServerConfig) {
    let mut sys = RealSystem::new(
        1024,
        config.host.as_str(),
        config.port,
        config.mount_dir.as_str(),
    );

    let replica_address = Address::new(
        config.replica_host,
        config.replica_port,
        "server".to_owned(),
    );

    let server = ServerProcess::new_with_replica(replica_address);
    let wrapper = sys.add_process(server, "server".to_owned());

    sys.spawn(async move {
        // send request to download absent history from replica
        wrapper.sender.send(check_replica_request()).await.unwrap();
    });

    sys.run();
}
