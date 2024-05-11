use serde::{Deserialize, Serialize};

/// Represents config of the client.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub launch: String,
    pub login: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub server_host: String,
    pub server_port: u16,
    pub replica_host: String,
    pub replica_port: u16,
}

/// Represents config of the server.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub launch: String,
    pub host: String,
    pub port: u16,
    pub mount_dir: String,
    pub replica_host: String,
    pub replica_port: u16,
}

impl ClientConfig {
    pub fn from_file(filename: &str) -> Option<Self> {
        std::fs::File::open(filename)
            .ok()
            .and_then(|file| serde_yaml::from_reader(file).ok())
    }
}

impl ServerConfig {
    pub fn from_file(filename: &str) -> Option<Self> {
        std::fs::File::open(filename)
            .ok()
            .and_then(|file| serde_yaml::from_reader(file).ok())
    }
}
