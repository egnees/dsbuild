//! Definition of context-related objects.

use std::future::Future;

use log::warn;

use crate::{common::message::RoutedMessage, Address, Message};

use super::{
    network::{self, NetworkRequest},
    process::{Output, ToSystemMessage},
};

/// Represents context of system in the real mode.
#[derive(Clone)]
pub(crate) struct RealContext {
    pub(crate) output: Output,
    pub(crate) address: Address,
}

impl RealContext {
    /// Send local message.
    pub fn send_local(&self, message: Message) {
        let sender = self.output.local.clone();
        tokio::spawn(async move {
            let result = sender.send(message).await;

            if let Err(info) = result {
                warn!("Can not send local message: {}", info);
            }
        });
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, delay will be override.
    pub fn set_timer(&self, name: &str, delay: f64) {
        self.output
            .timer_mngr
            .lock()
            .unwrap()
            .set_timer(name.to_owned(), delay, true);
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, delay will not be override.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        self.output
            .timer_mngr
            .lock()
            .unwrap()
            .set_timer(name.to_owned(), delay, false);
    }

    /// Cancel timer with specified name.
    pub fn cancel_timer(&self, name: &str) {
        self.output.timer_mngr.lock().unwrap().cancel_timer(name);
    }

    /// Send network message.
    pub fn send(&self, msg: Message, dst: Address) {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to: dst,
        };
        let sender = self.output.network.clone();
        tokio::spawn(async move {
            let result = sender.send(NetworkRequest::SendMessage(msg)).await;

            if let Err(info) = result {
                warn!("Can not send network message: {}", info);
            }
        });
    }

    /// Send network message reliable.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered.
    /// - Ok if message was delivered
    pub async fn send_reliable(&self, msg: Message, dst: Address) -> Result<(), String> {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to: dst,
        };

        network::send_message_reliable(msg).await
    }

    /// Spawn asynchronous activity.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }

    /// Sleep for some time (sec.).
    pub async fn sleep(&self, duration: f64) {
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(duration)).await;
    }

    /// Stop the process.
    pub fn stop(self) {
        tokio::spawn(async move {
            self.output
                .system
                .send(ToSystemMessage::ProcessStopped(self.address.process_name))
                .await
                .unwrap()
        });
    }
}
