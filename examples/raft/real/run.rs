use std::env;

use cfg::StructuredConfig;
use dsbuild::Address;
use io::process_io;

pub mod cfg;
pub mod http;
pub mod io;
pub mod register;

fn main() -> Result<(), String> {
    // enable logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    // read args
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        return Err(format!(
            "Usage: {} <config_file> <replica> <system_files_path>",
            args[0]
        ));
    }

    // get config and corresponding info
    let config_filename = &args[1];
    let config =
        StructuredConfig::from_file(config_filename).ok_or("incorrect config".to_owned())?;
    let my_id: usize = args[2]
        .parse()
        .map_err(|_| "bad second argument".to_string())?;

    // get all addrs
    let addrs: Vec<_> = config
        .inner_net
        .iter()
        .map(|socket_addr| {
            Address::new(
                socket_addr.ip().to_string(),
                socket_addr.port(),
                "raft".to_owned(),
            )
        })
        .collect();

    let my_addr = &addrs[my_id];
    let storage_mount = &args[3];

    // create node
    let mut node = dsbuild::RealNode::new(&my_addr.host, my_addr.port, storage_mount);

    // create process
    let proc = raft::proc::RaftProcess::new(addrs, my_id, 0.1);

    // add process on node and get I/O wrapper
    let proc_io = node.add_process(proc, "raft".to_owned());

    // get my listen address
    let my_listen_addr = config.listen_net[my_id];

    // spawn I/O activity
    node.spawn(process_io(
        my_id,
        my_listen_addr,
        proc_io.sender,
        proc_io.receiver,
        config.listen_net,
    ));

    // run spawned activity
    node.run();

    Ok(())
}
