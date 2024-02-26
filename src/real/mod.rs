//! Definition of structures and functions, which are used in [`real mode`][`crate::RealSystem`].

pub mod context;
mod events;
mod network;
mod process_manager;
mod time;

pub mod system;

#[cfg(test)]
mod tests;
