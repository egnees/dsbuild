//! Definitions which are used in [network][`super`] submodule.

use crate::common::message::Message;

/// Specifies process address.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Address {
    /// Specifies host, 
    /// which is used to deliver messages 
    /// to the [`crate::real_mode::system::System`] instance 
    /// through the network.
    pub host: String,
    
    /// Specifies port, 
    /// which is used to deliver messages 
    /// to the [`crate::real_mode::system::System`] instance 
    /// through  the network.
    pub port: u16,
    
    /// Specifies destination process name
    /// inside of the [`crate::real_mode::system::System`] instance.
    pub process_name: String,
}

/// Used to pass requests to some object, 
/// which implements [`AsyncMessenger`][`super::messenger::AsyncMessenger`] trait.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSendRequest {
    /// Address of process, which sends request.
    pub sender_address: Address,
    /// Address of process, which will receive request.
    pub receiver_address: Address,
    /// Passed message.
    pub message: Message,
}

/// Used to pass responses on [requests][`ProcessSendRequest`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSendResponse {
    /// Response status, 
    /// which indicates whether request was successfully sended or not.
    pub status: String,
}
