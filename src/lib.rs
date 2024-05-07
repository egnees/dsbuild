//! Framework for building distributed systems with support for
//! [DSLab MP](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html)
//! simulation-based testing.

// Add warnings for missing public documentation.
#![warn(missing_docs)]

// Add warnings for missing in private documentation (disabled for now).
// #![warn(clippy::missing_docs_in_private_items)]

mod real;

// Re-export public entities.
pub use real::system::System as RealSystem;

mod virt;

// Re-export public entities.
pub use virt::system::System as VirtualSystem;

mod common;
pub use common::storage;

// Re-export public entities.
pub use common::{
    context::Context,
    message::Message,
    process::{Address, IOProcessWrapper, Process, ProcessGuard, ProcessWrapper},
    tag::Tag,
};
