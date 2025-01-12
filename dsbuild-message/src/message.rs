use std::io::Read;

use serde::{Deserialize, Serialize};

/// Represents message, which is used by [processes][crate::Process] to communicate
/// with each other by the network.
#[derive(Serialize, Clone, Eq, Hash, PartialEq, PartialOrd, Ord, Debug)]
pub struct Message {
    tip: String,
    data: Vec<u8>,
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

use crate::Tipped;

impl From<Message> for DSLabMessage {
    fn from(message: Message) -> Self {
        let mut payload = String::new();
        message.get_raw_data().read_to_string(&mut payload).unwrap();
        DSLabMessage::new(
            message.get_tip(),
            &String::from_utf8_lossy(message.data.as_slice()).to_string(),
        )
    }
}

impl From<DSLabMessage> for Message {
    fn from(value: DSLabMessage) -> Self {
        Message::new_raw(&value.tip, value.data.as_bytes()).unwrap()
    }
}

////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////

impl<T: Tipped> From<T> for Message {
    fn from(value: T) -> Self {
        Self::new(T::TIP, &value).unwrap()
    }
}
