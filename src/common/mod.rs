//! Definition of structures and functions,
//! which are used by [`real`][`crate::RealNode`] and [`virtual`][`crate::Sim`] systems.

pub mod context;
pub mod fs;
pub mod message;
pub mod network;
pub mod process;
pub mod storage;
pub mod tag;

#[cfg(test)]
mod tests;
