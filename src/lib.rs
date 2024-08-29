#![warn(missing_docs)]

//! DSbuild is a high-level framework aimed to provide foundation for building
//! distributed systems with support for simulation based testing. The framework
//! guarantees system behaviour will be consistent in both simulation and real
//! modes, which can significantly easy testing and debugging.

////////////////////////////////////////////////////////////////////////////////
mod real;

// Re-export public entities.
pub use real::io::IOProcessWrapper;
pub use real::node::Node as RealNode;

////////////////////////////////////////////////////////////////////////////////

mod sim;

// Re-export public entities.
pub use sim::system::Sim as VirtualSystem;

////////////////////////////////////////////////////////////////////////////////

mod common;
pub use common::storage;

// Re-export public entities.
pub use common::{
    context::Context,
    message::Message,
    network::{SendError, SendResult},
    process::{Address, Process, ProcessGuard, ProcessWrapper},
    tag::Tag,
};
