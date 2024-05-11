//! Definition of context-related objects.

use std::{future::Future, time::Duration};

use dslab_async_mp::storage::result::{StorageError, StorageResult};
use tokio::{select, sync::oneshot};

use crate::{
    common::{
        file::File,
        message::RoutedMessage,
        network::{SendError, SendResult},
        tag::Tag,
    },
    Address, Message,
};

use std::io::ErrorKind;

use super::{
    network::{self, NetworkRequest},
    process::{InteractionBlock, ToSystemMessage},
};

/// Represents context of system in the real mode.
#[derive(Clone)]
pub(crate) struct RealContext {
    pub(crate) output: InteractionBlock,
    pub(crate) address: Address,
    pub(crate) mount_dir: String,
}

impl RealContext {
    /// Send local message.
    pub fn send_local(&self, message: Message) {
        let sender = self.output.local.clone();
        let result = sender.try_send(message);
        if let Err(info) = result {
            log::warn!("can not send local message: {}", info);
        }
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
            tag: None,
        };
        let sender = self.output.network.clone();
        tokio::spawn(async move {
            let result = sender.send(NetworkRequest::SendMessage(msg)).await;

            if let Err(info) = result {
                log::warn!("Can not send network message: {}", info);
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
    pub async fn send_with_ack(&self, msg: Message, dst: Address, timeout: f64) -> SendResult<()> {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to: dst,
            tag: None,
        };

        network::send_message_with_ack_timeout(msg, timeout).await
    }

    /// See [`crate::common::context::Context::send_with_tag`].
    pub async fn send_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> SendResult<()> {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to,
            tag: Some(tag),
        };

        network::send_message_with_ack_timeout(msg, timeout).await
    }

    /// See [`crate::common::context::Context::send_recv_with_tag`].
    pub async fn send_recv_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> SendResult<Message> {
        let (sender, receiver) = oneshot::channel();

        let output = self.output.clone();
        let from = self.address.clone();

        output
            .message_waiters
            .lock()
            .unwrap()
            .entry(tag)
            .or_default()
            .push(sender);

        let timeout = Duration::from_millis((timeout * 1000.0) as u64);

        let send_future = async move {
            network::send_message_with_ack(RoutedMessage {
                msg,
                from,
                to,
                tag: None,
            })
            .await?;

            receiver.await.map_err(|_| SendError::NotSent)
        };

        select! {
            result = send_future => result,
            _ = tokio::time::sleep(timeout) => Err(SendError::Timeout)
        }
    }

    /// Spawn asynchronous activity.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
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

    /// Check if file exists.
    pub async fn file_exists(&self, name: &str) -> StorageResult<bool> {
        let mount_dir = self.mount_dir.clone();
        match async_std::fs::File::open(mount_dir + "/" + name).await {
            Ok(_) => Ok(true),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => Ok(false),
                _ => Err(StorageError::Unavailable),
            },
        }
    }

    /// Create file with specified name.
    pub async fn create_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        let mount_dir = self.mount_dir.clone();

        if self.file_exists(name).await? {
            return Err(StorageError::AlreadyExists);
        }

        async_std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(mount_dir + "/" + name)
            .await
            .map_err(|_| StorageError::Unavailable)
            .map(File::RealFile)
    }

    /// Open file with specified name.
    pub async fn open_file<'a>(&'a self, name: &'a str) -> StorageResult<File> {
        let mount_dir = self.mount_dir.clone();

        async_std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(mount_dir + "/" + name)
            .await
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => StorageError::NotFound,
                _ => StorageError::Unavailable,
            })
            .map(File::RealFile)
    }
}
