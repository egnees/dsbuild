//! Definition of network-related structures.

/// Represents error of `send message` operation.
pub type SendError = dslab_async_mp::network::result::SendError;

/// Represents result of `send message` operation.
pub type SendResult<T> = dslab_async_mp::network::result::SendResult<T>;
