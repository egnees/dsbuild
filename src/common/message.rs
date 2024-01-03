//! Definition of Message which could be passed through network.

use serde::{Deserialize, Serialize};

/// Represents a message.
#[derive(Serialize, Clone, Eq, Hash, PartialEq, PartialOrd, Ord, Debug)]
pub struct Message {
    tip: String,
    data: Vec<u8>,
}

impl Message {
    pub fn new<T>(tip: &str, data: &T) -> Result<Self, String>
    where
        T: Serialize,
    {
        let data_serialized = serde_json::to_vec(data).map_err(|err| err.to_string())?;

        Ok(Self {
            tip: tip.to_string(),
            data: data_serialized,
        })
    }

    pub fn new_raw(tip: &str, data: &[u8]) -> Result<Self, String> {
        Ok(Self {
            tip: tip.to_string(),
            data: data.to_vec(),
        })
    }

    pub fn borrow_new<T>(tip: &str, data: T) -> Result<Self, String>
    where
        T: Serialize,
    {
        let data_serialized = serde_json::to_vec(&data).map_err(|err| err.to_string())?;

        Ok(Self {
            tip: tip.to_string(),
            data: data_serialized,
        })
    }

    pub fn get_tip(&self) -> &String {
        &self.tip
    }

    pub fn get_raw_data(&self) -> &[u8] {
        &self.data
    }

    pub fn get_data<'a, T>(&'a self) -> Result<T, String>
    where
        T: Deserialize<'a>,
    {
        let data_deserealized =
            serde_json::from_slice::<'a, T>(self.data.as_slice()).map_err(|err| err.to_string())?;

        Ok(data_deserealized)
    }
}

use dslab_mp::message::Message as DSlabMessage;

impl From<DSlabMessage> for Message {
    fn from(msg: DSlabMessage) -> Self {
        Self {
            tip: msg.tip.clone(),
            data: msg.data.clone().into(),
        }
    }
}

impl From<Message> for DSlabMessage {
    fn from(msg: Message) -> Self {
        DSlabMessage {
            tip: msg.tip.clone(),
            data: std::str::from_utf8(msg.data.as_slice())
                .expect("Can not cast Message data to str")
                .to_string(),
        }
    }
}
