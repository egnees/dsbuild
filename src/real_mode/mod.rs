//! Definition of structures and functions, which are used in [`real mode`][`crate::RealSystem`].

mod events;
mod network;
mod process_manager;
mod real_context;
mod time;

pub mod real_system;

#[cfg(test)]
mod tests;
