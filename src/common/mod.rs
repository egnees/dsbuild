//! Definition of structures and functions,
//! which are used by [`real`][`crate::RealSystem`] and [`virtual`][`crate::VirtualSystem`] systems.

pub mod context;
pub mod file;
pub mod network;
pub mod process;
pub mod storage;
pub mod tag;

pub mod message;

#[cfg(test)]
mod tests;
