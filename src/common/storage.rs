//! Definition of storage errors.

/// Represents errors which appears when working with storage.
pub type StorageError = dslab_async_mp::storage::result::StorageError;

/// Represents result for operations with storage.
pub type StorageResult<T> = Result<T, StorageError>;
