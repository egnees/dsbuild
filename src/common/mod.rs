//! Definition of structures and functions,
//! which are used by [`real`][`crate::RealSystem`] and [`virtual`][`crate::VirtualSystem`] systems.

pub mod context;
pub mod process;

pub mod message;

#[cfg(test)]
mod tests;
