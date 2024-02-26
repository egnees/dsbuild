//! Definition of structures and functions, which are used in [`virtual mode`][`crate::VirtualSystem`].

pub mod context;
mod node_manager;
mod process_wrapper;

pub mod system;

#[cfg(test)]
mod tests;
