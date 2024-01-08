//! Framework for building distributed systems with support for
//! [DSLab MP](https://osukhoroslov.github.io/dslab/docs/dslab_mp/index.html) simulation-based testing.

// Add warnings for missing public documentation.
#![warn(missing_docs)]

// Add warnings for missing in private documentation (disabled for now).
// #![warn(clippy::missing_docs_in_private_items)]

mod real_mode;

// Re-export public entities.
pub use real_mode::real_system::{Address, AddressResolvePolicy, RealSystem, RealSystemConfig};

mod virtual_mode;

// Re-export public entities.
pub use virtual_mode::virtual_system::VirtualSystem;

mod common;

// Re-export public entities.
pub use common::{
    context::Context,
    message::Message,
    process::{Process, ProcessGuard, ProcessWrapper},
};

// Public module.
pub mod process_lib;

// Examples
pub mod examples;
