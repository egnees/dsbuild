use std::env;

use cfg::{ClientConfig, ServerConfig};

mod cfg;
mod client;
mod server;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(format!("Usage: {} <config_file>", args[0]));
    }
    let file = args[1].to_owned();
    if let Some(config) = ClientConfig::from_file(&file) {
        client::run_client_with_config(config);
    } else if let Some(config) = ServerConfig::from_file(&file) {
        server::run_server_with_config(config);
    } else {
        return Err("invalid config file".to_owned());
    }
    Ok(())
}
