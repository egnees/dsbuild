//! Definition of [`Message`] which could be passed through network.

use crate::Address;
pub use dsbuild_message::Message;

////////////////////////////////////////////////////////////////////////////////

/// Represents message tag.
///
/// For more details, see [`send_with_tag`][crate::Context::send_with_tag] and
/// [`send_recv_with_tag`][crate::Context::send_recv_with_tag] documentation.
pub type Tag = u64;

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub(crate) struct RoutedMessage {
    pub msg: Message,
    pub from: Address,
    pub to: Address,
    pub tag: Option<Tag>,
}
