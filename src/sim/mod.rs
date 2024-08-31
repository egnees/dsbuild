//! Definition of structures and functions, which are used in [`virtual mode`][`crate::Sim`].

pub mod context;
pub mod fs;
mod node;
mod process;

mod send_future;

pub mod system;

#[cfg(test)]
mod tests;
