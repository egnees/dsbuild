//! Definition of structures and functions, which are used in [`virtual mode`][`crate::VirtualSystem`].

pub mod context;
pub mod file;
mod node;
mod process;

mod send_future;

pub mod system;

#[cfg(test)]
mod tests;
