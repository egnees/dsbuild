//! Definition of virtual mode context.

use std::{cell::RefCell, future::Future, rc::Rc};

use crate::common::{
    fs::{File, FsResult},
    message::{Message, Tag},
    network::{SendError, SendResult},
    process::Address,
};
use dslab_async_mp::process::context::Context as DSLabContext;

use super::{
    fs::FileWrapper,
    node::NodeManager,
    send_future::{SendFuture, Sf},
};

/// Represents context in virtual mode.
/// Responsible for user-simulation interaction.
/// Serves as a proxy between user and underlying
/// [DSLab MP simulation](https://github.com/osukhoroslov/dslab/tree/main/crates/dslab-mp),
/// uses corresponding `DSLabContext` methods.
#[derive(Clone)]
pub(crate) struct VirtualContext {
    pub dslab_ctx: DSLabContext,
    pub node_manager: Rc<RefCell<NodeManager>>,
}

impl VirtualContext {
    /// Send local message.
    pub fn send_local(&self, message: Message) {
        self.dslab_ctx.send_local(message.into());
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, delay will be override.
    pub fn set_timer(&self, name: &str, delay: f64) {
        self.dslab_ctx.set_timer(name, delay);
    }

    /// Set timer with specified name and delay.
    /// If such timer already exists, nothing happens.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        self.dslab_ctx.set_timer_once(name, delay);
    }

    /// Cancel timer with specified name.
    pub fn cancel_timer(&self, name: &str) {
        self.dslab_ctx.cancel_timer(name);
    }

    /// Send message to specified address.
    pub fn send(&self, msg: Message, dst: Address) {
        match self.node_manager.borrow().get_full_process_name(&dst) {
            Ok(full_process_name) => {
                self.dslab_ctx.send(msg.into(), &full_process_name);
            }
            Err(err) => {
                log::warn!("Message not sent: {}", err);
            }
        }
    }

    /// Send network message reliable with specified timeout.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered with specified timeout.
    /// - Ok if message was delivered
    pub fn send_with_ack(&self, msg: Message, dst: Address, timeout: f64) -> Sf<SendResult<()>> {
        let process_name = self.node_manager.borrow().get_full_process_name(&dst);

        let ctx = self.dslab_ctx.clone();
        SendFuture::from_future(async move {
            if let Ok(process_name) = process_name {
                Ok(ctx
                    .send_with_ack(msg.into(), &process_name, timeout)
                    .await?)
            } else {
                Err(SendError::NotSent)
            }
        })
    }

    /// See [`crate::common::context::Context::send_with_tag`].
    pub fn send_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> Sf<SendResult<()>> {
        let process_name = self.node_manager.borrow().get_full_process_name(&to);

        let ctx = self.dslab_ctx.clone();
        SendFuture::from_future(async move {
            if let Ok(process_name) = process_name {
                Ok(ctx
                    .send_with_tag(msg.into(), tag, &process_name, timeout)
                    .await?)
            } else {
                Err(SendError::NotSent)
            }
        })
    }

    /// See [`crate::common::context::Context::send_recv_with_tag`].
    pub fn send_recv_with_tag(
        &self,
        msg: Message,
        tag: Tag,
        to: Address,
        timeout: f64,
    ) -> Sf<SendResult<Message>> {
        let process_name = self.node_manager.borrow().get_full_process_name(&to);

        let ctx = self.dslab_ctx.clone();
        SendFuture::from_future(async move {
            if let Ok(process_name) = process_name {
                Ok(ctx
                    .send_recv_with_tag(msg.into(), tag, &process_name, timeout)
                    .await
                    .map(|msg| msg.into())?)
            } else {
                Err(SendError::NotSent)
            }
        })
    }

    /// Spawn asynchronous activity.
    pub fn spawn(&self, future: impl Future<Output = ()>) {
        self.dslab_ctx.spawn(future)
    }

    /// Stop the process.
    pub fn stop(self) {
        // Does not need to do anything here.
    }

    /// Create file with specified name.
    pub fn create_file<'a>(&'a self, name: &'a str) -> Sf<'a, FsResult<File>> {
        let future = async move {
            Ok(self
                .dslab_ctx
                .create_file(name)
                .map(|file| File::from_sim(FileWrapper { file }))?)
        };

        SendFuture::from_future(future)
    }

    /// Delete file with specified name.
    pub fn delete_file<'a>(&'a self, name: &'a str) -> Sf<'a, FsResult<()>> {
        let future = async move { Ok(self.dslab_ctx.delete_file(name)?) };

        SendFuture::from_future(future)
    }

    /// Check if file exists.
    pub fn file_exists<'a>(&'a self, name: &'a str) -> Sf<'a, FsResult<bool>> {
        SendFuture::from_future(async move { Ok(self.dslab_ctx.file_exists(name)?) })
    }

    /// Open file.
    pub fn open_file<'a>(&'a self, name: &'a str) -> Sf<'a, FsResult<File>> {
        SendFuture::from_future(async move {
            Ok(self
                .dslab_ctx
                .open_file(name)
                .map(|file| File::from_sim(FileWrapper { file }))?)
        })
    }

    /// Get current simulation time.
    pub fn time(&self) -> f64 {
        self.dslab_ctx.time()
    }
}

/// [`VirtualContext`] wont be shared between threads,
/// but Rust rules require it to be [`Send`] + [`Sync`],
/// because it will be used inside of futures.
/// This futures will not and can not be shared between threads,
/// but Rust can not know it in compile time.
unsafe impl Send for VirtualContext {}
unsafe impl Sync for VirtualContext {}

#[cfg(test)]
mod tests {
    use crate::{Address, Context, Message, Process, Sim};

    #[test]
    fn local_messages() {
        struct LocalEchoer {}

        impl Process for LocalEchoer {
            fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
                ctx.send_local(msg);
                Ok(())
            }

            fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
                unimplemented!()
            }

            fn on_message(
                &mut self,
                _msg: Message,
                _from: Address,
                _ctx: Context,
            ) -> Result<(), String> {
                unimplemented!()
            }
        }

        let mut sim = Sim::new(123);
        sim.add_node("node", "192.168.13.2", 10101);
        let process = LocalEchoer {};
        sim.add_process("proc", process, "node");
        sim.send_local_message("proc", "node", "message".into());
        sim.step_until_no_events();
        let messages = sim.read_local_messages("proc", "node").unwrap();
        assert_eq!(messages, vec!["message".into()]);
    }
}
