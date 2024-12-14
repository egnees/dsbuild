use std::{future::Future, pin::Pin};

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
pub struct SendFuture<'a, T>
where
    T: Send,
{
    future: Pin<Box<dyn Future<Output = T> + 'a>>,
}

impl<'a, T> SendFuture<'a, T>
where
    T: Send,
{
    pub fn from_future(future: impl Future<Output = T> + 'a) -> Pin<Box<Self>> {
        Box::pin(SendFuture {
            future: Box::pin(future),
        })
    }
}

impl<T> Future for SendFuture<'_, T>
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
unsafe impl<T> Send for SendFuture<'_, T> where T: Send {}

/// Represents alias on [`Send`] future.
pub type Sf<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
