//! Definitions which are used in [network][`super`] submodule.

use crate::common::{message::Message, process::Address};

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
