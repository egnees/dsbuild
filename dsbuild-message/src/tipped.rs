use serde::{Deserialize, Serialize};

/// Requires [`Tipped::TIP`] to auto-implement
/// [`From<Message>`] and [`Into<Message>`] traits.
pub trait Tipped: Serialize + for<'a> Deserialize<'a> {
    /// Represents tip of the message.
    const TIP: &str;
}
