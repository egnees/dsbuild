use std::net::{SocketAddr, ToSocketAddrs};

use serde::Deserialize;

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Deserialize)]
struct Config {
    pub inner_net: Vec<String>,
    pub listen_net: Vec<String>,
}

//////////////////////////////////////////////////////////////////////////////////////////

impl Config {
    pub fn from_file(filename: &str) -> Option<Self> {
        std::fs::File::open(filename)
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Sorted and serialized
pub struct StructuredConfig {
    pub inner_net: Vec<SocketAddr>,
    pub listen_net: Vec<SocketAddr>,
}

fn str_to_sock(s: &str) -> Option<SocketAddr> {
    s.to_socket_addrs().ok()?.next()
}

fn sock_addrs_from_str_list(list: &[String]) -> Option<Vec<SocketAddr>> {
    let all_socks = list.iter().map(|addr| str_to_sock(addr));
    if !all_socks.clone().all(|opt| opt.is_some()) {
        None
    } else {
        let mut all_socks = all_socks.map(|opt| opt.unwrap()).collect::<Vec<_>>();
        all_socks.sort();
        Some(all_socks)
    }
}

impl StructuredConfig {
    pub fn from_file(filename: &str) -> Option<Self> {
        let config = Config::from_file(filename)?;

        Some(Self {
            inner_net: sock_addrs_from_str_list(&config.inner_net)?,
            listen_net: sock_addrs_from_str_list(&config.listen_net)?,
        })
    }
}
