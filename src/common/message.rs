//! Definition of [`Message`] which could be passed through network.

use serde::{Deserialize, Serialize};

////////////////////////////////////////////////////////////////////////////////

/// Represents message, which is used by [processes][crate::Process] to communicate
/// with each other by the network.
#[derive(Serialize, Clone, Eq, Hash, PartialEq, PartialOrd, Ord, Debug)]
pub struct Message {
    tip: String,
    data: Vec<u8>,
}

////////////////////////////////////////////////////////////////////////////////

/// Represents message tag.
///
/// For more details, see [`send_with_tag`][crate::Context::send_with_tag] and
/// [`send_recv_with_tag`][crate::Context::send_recv_with_tag] documentation.
pub type Tag = u64;

#[derive(Clone)]
pub(crate) struct RoutedMessage {
    pub msg: Message,
    pub from: Address,
    pub to: Address,
    pub tag: Option<Tag>,
}

impl Message {
    /// Create a new message with specified tip and data, which will be serialized and passed
    /// inside of the message.
    pub fn new<T>(tip: &str, data: &T) -> Result<Self, String>
    where
        T: Serialize,
    {
        serde_json::to_vec(data)
            .map_err(|err| "Can not create message: ".to_owned() + err.to_string().as_str())
            .map(|data| Self {
                tip: tip.to_string(),
                data,
            })
    }

    /// Create a new message with specified tip and raw data.
    pub fn new_raw(tip: &str, data: &[u8]) -> Result<Self, String> {
        Ok(Self {
            tip: tip.to_string(),
            data: data.to_vec(),
        })
    }

    /// Get message's tip.
    pub fn get_tip(&self) -> &String {
        &self.tip
    }

    /// Get message's raw data.
    pub fn get_raw_data(&self) -> &[u8] {
        &self.data
    }

    /// Returns deserialized message's data of template type,
    /// which must implement [`Deserialize`] trait.
    pub fn get_data<'a, T>(&'a self) -> Result<T, String>
    where
        T: Deserialize<'a>,
    {
        serde_json::from_slice::<'a, T>(self.data.as_slice()).map_err(|err| err.to_string())
    }
}

////////////////////////////////////////////////////////////////////////////////

use dslab_async_mp::network::message::Message as DSLabMessage;

use crate::Address;

impl From<DSLabMessage> for Message {
    fn from(msg: DSLabMessage) -> Self {
        Self {
            tip: msg.tip.clone(),
            data: msg.data.clone().into(),
        }
    }
}

impl From<Message> for DSLabMessage {
    fn from(msg: Message) -> Self {
        DSLabMessage {
            tip: msg.tip.clone(),
            data: String::from_utf8_lossy(msg.data.as_slice()).to_string(),
        }
    }
}

impl From<String> for Message {
    fn from(value: String) -> Self {
        Message::new("info", &value).unwrap()
    }
}

impl From<&str> for Message {
    fn from(value: &str) -> Self {
        Message::new("info", &value).unwrap()
    }
}
