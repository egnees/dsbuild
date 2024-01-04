//! Defines modules in which collected structures and methods dedicated to work with network.

pub mod defs;
pub mod grpc_messenger;
pub mod manual_resolver;
pub mod messenger;
pub mod network_manager;
pub mod resolver;

#[cfg(test)]
mod tests;
