//! Definition of asynchronous messenger [`AsyncMessenger`] trait.

use crate::real_old::events::Event;

use super::defs::*;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

/// Asynchronous messenger trait, which is used to send messages between user processes.
#[async_trait]
pub trait AsyncMessenger {
    /// Creates [future][`core::future::Future`], which execution will lead to
    /// sending [message][`crate::common::message::Message`] between [user processes][`crate::common::process::Process`]
    /// through network, based on specified `request`.
    ///
    /// # Returns
    ///
    /// Future, which execution will return [`Result`] consists of:
    /// -  [`ProcessSendResponse`] with [send status][`ProcessSendResponse::status`] in case of successful sending
    /// -  [`String`] will error in case of unsuccessful sending
    async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String>;

    /// Creates [future][`core::future::Future`], which execution 
    /// will create listener of incoming [`messages`][`crate::common::message::Message`]
    /// from other [user processes][`crate::common::process::Process`].
    async fn listen(host: String, port: u16, pass_to: Sender<Event>) -> Result<(), String>;
}
