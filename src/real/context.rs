//! Definition of context-related objects.

use std::{
    future::Future,
    io::SeekFrom,
    sync::{Arc, Mutex},
};

use async_std::io::{prelude::SeekExt, ReadExt, WriteExt};

use crate::{
    common::{
        message::RoutedMessage,
        storage::{CreateFileError, ReadError, WriteError, MAX_BUFFER_SIZE},
    },
    Address, Message,
};

use std::io::ErrorKind;

use super::{
    network::{self, NetworkRequest},
    process::{Output, ToSystemMessage},
    storage::FileManager,
};

/// Represents context of system in the real mode.
#[derive(Clone)]
pub(crate) struct RealContext {
    pub(crate) output: Output,
    pub(crate) address: Address,
    pub(crate) file_manager: Arc<Mutex<FileManager>>,
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
    pub async fn send_reliable(&self, msg: Message, dst: Address) -> Result<(), String> {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to: dst,
        };

        network::send_message_reliable(msg).await
    }

    /// Send network message reliable.
    /// If message will not be delivered in specified timeout,
    /// error will be returned.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered in specified timeout.
    /// - Ok if message was delivered
    pub async fn send_reliable_timeout(
        &self,
        msg: Message,
        dst: Address,
        timeout: f64,
    ) -> Result<(), String> {
        let msg = RoutedMessage {
            msg,
            from: self.address.clone(),
            to: dst,
        };

        network::send_message_reliable_timeout(msg, timeout).await
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

    /// Read file from the specified offset into the specified buffer.
    ///
    /// # Returns
    /// The number of read bytes.
    pub async fn read(
        &self,
        file: &str,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, ReadError> {
        if buf.len() > MAX_BUFFER_SIZE {
            panic!(
                "size of buffer exceeds max size: {} exceeds {}",
                buf.len(),
                MAX_BUFFER_SIZE
            );
        }

        let file_lock = self
            .file_manager
            .lock()
            .unwrap()
            .get_file_lock(file)
            .ok_or(ReadError::FileNotFound)?;

        // Exclusive lock on the file will be dropped when file will be read.
        let _file_guard = file_lock.lock().await;

        let mut file = async_std::fs::File::open(
            self.file_manager.lock().unwrap().get_mount_dir().to_owned() + "/" + file,
        )
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => ReadError::FileNotFound,
            _ => ReadError::Unavailable,
        })?;

        file.seek(SeekFrom::Start(offset.try_into().unwrap()))
            .await
            .map_err(|_| ReadError::Unavailable)?;

        file.read(buf).await.map_err(|_| ReadError::Unavailable)
    }

    /// Append data to file.
    pub async fn append(&self, file: &str, data: &'static [u8]) -> Result<(), WriteError> {
        let file_lock = self
            .file_manager
            .lock()
            .unwrap()
            .get_file_lock(file)
            .ok_or(WriteError::FileNotFound)?;

        // Exclusive lock on the file will be dropped when work with file will be done.
        let _file_guard = file_lock.lock().await;

        let mut file = async_std::fs::File::open(
            self.file_manager.lock().unwrap().get_mount_dir().to_owned() + "/" + file,
        )
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => WriteError::FileNotFound,
            _ => WriteError::Unavailable,
        })?;

        file.seek(SeekFrom::End(0))
            .await
            .map_err(|_| WriteError::Unavailable)?;

        file.write_all(data)
            .await
            .map_err(|_| WriteError::OutOfMemory)
    }

    /// Create file with specified name.
    pub async fn create_file(&self, name: &'static str) -> Result<(), CreateFileError> {
        let lock = self
            .file_manager
            .lock()
            .unwrap()
            .register_file(name.to_owned())
            .ok_or(CreateFileError::FileAlreadyExists)?;

        let _guard = lock.lock().await;

        // In case creation fails, then disk is unavailable and future working is UB.
        async_std::fs::File::create(
            self.file_manager.lock().unwrap().get_mount_dir().to_owned() + "/" + name,
        )
        .await
        .map_err(|e| match e.kind() {
            ErrorKind::AlreadyExists => CreateFileError::FileAlreadyExists,
            _ => CreateFileError::Unavailable,
        })
        .map(|_| ())
    }
}
