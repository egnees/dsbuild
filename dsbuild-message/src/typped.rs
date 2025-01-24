use serde::{Deserialize, Serialize};

/// Requires [`Typped::type`] to auto-implement
/// [`From<Message>`] and [`Into<Message>`] traits.
pub trait Typped: Serialize + for<'a> Deserialize<'a> {
    /// Represents type of the message.
    const TYPE: &str;
}
