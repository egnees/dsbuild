//! Library for building distributed systems in Rust
//! with support for testing and debugging in simulation.

// Add warnings for missing public documentation.
#![warn(missing_docs)]

// Add warnings for missing in private documentation (disabled for now).
// #![warn(clippy::missing_docs_in_private_items)]

pub mod common;

pub mod process_lib;

pub mod real_mode;

pub mod virtual_mode;
