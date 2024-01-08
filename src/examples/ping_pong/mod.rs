//! Definition of Ping-Pong ecosystem.
//!
//! There are two processes:
//! [`Pinger`][`crate::process_lib::ping::PingProcess`] and [`Ponger`][`crate::process_lib::pong::PongProcess`].
//!
//! Pinger sends consecutive ping messages to the pong process with specified delay and waits for the consecutive pong responses,
//! while the last pong response is not received.
//!
//! Ponger waits for the consecutive pong requests. If there are no pong requests for a while, then the process is stopped.

pub mod pinger;
pub mod ponger;
pub mod real;
pub mod sim;
