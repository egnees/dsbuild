//! Definition of structures and functions,
//! which are used by [`real`][`crate::RealSystem`] and [`virtual`][`crate::VirtualSystem`] systems.

pub mod context;
pub mod process;

pub mod message;

pub mod actions;

#[cfg(test)]
mod tests;
