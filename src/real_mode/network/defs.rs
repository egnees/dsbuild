//! Definitions which are used in [network][`super`] submodule.

use crate::common::message::Message;

/// Represents [`process`][`crate::Process`] address, which is used in
/// [`real system`][`crate::RealSystem`] to route [`network messages`][crate::Message].
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Address {
    /// Specifies host,
    /// which is used to deliver messages
    /// to the [real system][`crate::real_mode::real_system::RealSystem`] instance
    /// through the network.
    pub host: String,

    /// Specifies port,
    /// which is used to deliver messages
    /// to the [real system][`crate::real_mode::real_system::RealSystem`] instance
    /// through the network.
    pub port: u16,

    /// Specifies process name
    /// inside of the [real system][`crate::real_mode::real_system::RealSystem`] instance.
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
    /// Response message from receiver,
    /// which indicates whether request was accepted or not.
    ///
    /// Remark: this protocol is not used for now,
    /// because there is no way to talk process
    /// if received message was successful delivered or not.
    pub status: String,
}
