//! Definition of Message which could be passed through network.

/// TODO
/// Implement inner Message type to remove dependency on dslab's interface.

use serde::{Serialize, Deserialize};

/// Represents a message.
#[derive(Serialize, Clone, Eq, Hash, PartialEq, PartialOrd, Ord, Debug)]
pub struct Message {
    tip: String,
    data: Vec<u8>,
}

impl Message {
    
    pub fn new<T>(tip: &str, data: &T) -> Result<Self, String> 
    where
        T: Serialize
    {
        let data_serialized = serde_json::to_vec(data)
            .map_err(|err| err.to_string())?;

        Ok(Self {
            tip: tip.to_string(),
            data: data_serialized
        })
    }

    pub fn borrow_new<T>(tip: &str, data: T) -> Result<Self, String> 
    where
        T: Serialize
    {
        let data_serialized = serde_json::to_vec(&data)
            .map_err(|err| err.to_string())?;

        Ok(Self {
            tip: tip.to_string(),
            data: data_serialized
        })
    }

    pub fn get_tip(&self) -> &String {
        &self.tip
    }

    pub fn fetch_data<'a, T>(&'a self) -> Result<T, String> 
    where
        T: Deserialize<'a>
    {
        let data_deserealized = serde_json::from_slice::<'a, T>(self.data.as_slice())
            .map_err(|err| err.to_string())?;

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

impl Into<DSlabMessage> for Message {
    fn into(self) -> DSlabMessage {
        DSlabMessage {
            tip: self.tip.clone(),
            data: std::str::from_utf8(self.data.as_slice()).expect("Can not cast Message data to str").to_string()
        }
    }
}
