//! Definition of structures and functions, which are used in [`virtual mode`][`crate::VirtualSystem`].

mod process_wrapper;
mod virtual_context;

pub mod virtual_system;

#[cfg(test)]
mod tests;
