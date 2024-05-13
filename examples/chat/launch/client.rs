use chat::{start_io, ClientProcess};
use dsbuild::{Address, RealNode};

use crate::cfg::ClientConfig;

pub fn run_client_with_config(config: ClientConfig) {
    let mut sys = RealNode::new(1024, config.host.as_str(), config.port, "/tmp/");

    let server1_address = Address::new(config.server_host, config.server_port, "server".to_owned());
    let server2_address = Address::new(
        config.replica_host,
        config.replica_port,
        "server".to_owned(),
    );
    let self_address = Address::new(config.host, config.port, "client".to_owned());

    let name = config.login;
    let password = config.password;
    let client = ClientProcess::new_with_replica(
        server1_address,
        server2_address,
        self_address,
        name,
        password,
    );

    let wrapper = sys.add_process(client, "client".to_owned());
    sys.spawn(start_io(wrapper));

    sys.run();
}
