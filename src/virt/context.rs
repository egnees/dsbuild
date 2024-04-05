//! Definition of virtual mode context.

use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc};

use crate::common::{message::Message, process::Address};
use dslab_async_mp::context::Context as DSLabContext;
use log::warn;

use super::node::NodeManager;

/// Represents context in virtual mode.
/// Responsible for user-simulation interaction.
/// Serves as a proxy between user and underlying
/// [DSLab MP simulation](https://github.com/osukhoroslov/dslab/tree/main/crates/dslab-mp),
/// uses corresponding [`DSLab MP context`][DSLabContext] methods.
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
                self.dslab_ctx.send(msg.into(), full_process_name);
            }
            Err(err) => {
                warn!("Message not sent: {}", err);
            }
        }
    }

    /// Send network message reliable.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered.
    /// - Ok if message was delivered
    pub fn send_reliable(
        &self,
        msg: Message,
        dst: Address,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let process_name = match self.node_manager.borrow().get_full_process_name(&dst) {
            Ok(full_process_name) => Some(full_process_name),
            Err(_) => None,
        };

        let ctx = self.dslab_ctx.clone();

        SendFuture::from_future(async move {
            if let Some(process_name) = process_name {
                ctx.send_reliable(msg.into(), process_name).await
            } else {
                Err(format!("Message not sent: bad dst address {:?}", dst))
            }
        })
    }

    /// Send network message reliable with specified timeout.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered with specified timeout.
    /// - Ok if message was delivered
    pub fn send_reliable_timeout(
        &self,
        msg: Message,
        dst: Address,
        timeout: f64,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let process_name = match self.node_manager.borrow().get_full_process_name(&dst) {
            Ok(full_process_name) => Some(full_process_name),
            Err(_) => None,
        };

        let ctx = self.dslab_ctx.clone();

        SendFuture::from_future(async move {
            if let Some(process_name) = process_name {
                ctx.send_reliable_timeout(msg.into(), process_name, timeout)
                    .await
            } else {
                Err(format!("message not sent: bad dst address {:?}", dst))
            }
        })
    }

    /// Spawn asynchronous activity.
    pub fn spawn(&self, future: impl Future<Output = ()>) {
        self.dslab_ctx.spawn(future);
    }

    /// Async sleep for some time (sec.).
    ///
    /// Explicitly returns [`Send`] future,
    /// besides future will not be shared between threads by design.
    /// See [`SendFuture`] for more details.
    pub fn sleep(&self, duration: f64) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let ctx = self.dslab_ctx.clone();
        SendFuture::from_future(async move { ctx.sleep(duration).await })
    }

    /// Stop the process.
    pub fn stop(self) {
        // Does not need to do anything here.
    }
}

/// [`VirtualContext`] wont be shared between threads,
/// but Rust rules require it to be [`Send`] + [`Sync`],
/// because it will be used inside of futures.
/// This futures will not and can not be shared between threads,
/// but Rust can not know it in compile time.
unsafe impl Send for VirtualContext {}
unsafe impl Sync for VirtualContext {}

/// Represents future which formally satisfies [`Send`] requirement.
/// [`SendFuture`] can not and will not be shared between threads,
/// but Rust rules require it to be [`Send`].
///
/// As [`VirtualContext`] methods use not [`Send`] + [`Sync`] elements,
/// futures which will use this methods will not satisfy [`Send`] trait,
/// because of that user can not spawn such futures,
/// although they will not be shared between threads.
/// To make it possible, [`SendFuture`] exists.
/// It formally implements [`Send`] trait.
struct SendFuture<T>
where
    T: Send,
{
    future: Pin<Box<dyn Future<Output = T>>>,
}

impl<T> SendFuture<T>
where
    T: Send,
{
    fn from_future(future: impl Future<Output = T> + 'static) -> Pin<Box<Self>> {
        Box::pin(SendFuture {
            future: Box::pin(future),
        })
    }
}

impl<T> Future for SendFuture<T>
where
    T: Send,
{
    type Output = T;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

/// Formally implementation of [`Send`] trait,
/// besides [`SendFuture`] will not be shared between threads.
unsafe impl<T> Send for SendFuture<T> where T: Send {}
