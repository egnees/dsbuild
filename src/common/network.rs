//! Definition of network-related structures.

use dslab_async_mp::network::result::SendError as DSLabSendError;

////////////////////////////////////////////////////////////////////////////////

/// Represents error type of [send][crate::Context::send] operation.
#[derive(Debug, Clone, PartialEq)]
pub enum SendError {
    /// Message was not acknowledged in the given time.
    Timeout,
    /// Message was not sent.
    NotSent,
}

impl From<DSLabSendError> for SendError {
    fn from(value: DSLabSendError) -> Self {
        match value {
            DSLabSendError::Timeout => Self::Timeout,
            DSLabSendError::NotSent => Self::NotSent,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Represents result of [send][crate::Context::send] operation.
pub type SendResult<T> = Result<T, SendError>;
