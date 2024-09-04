//! Manipulations with files.

use std::io::SeekFrom;

use crate::sim::fs::FileWrapper;
use async_std::{
    fs::File as RealFile,
    io::{prelude::SeekExt, ReadExt, WriteExt},
};

use dslab_async_mp::storage::result::StorageError as DSLabFsError;

////////////////////////////////////////////////////////////////////////////////

enum FileVariant {
    SimulationFile(FileWrapper),
    RealFile(RealFile),
}

/// Abstraction over the file.
///
/// Process can create, open, read and write to file.
/// To [create][crate::Context::create_file] or [open][crate::Context::open_file] file
/// refer to the corresponding [`Context`][crate::Context] methods.
pub struct File(FileVariant);

impl File {
    pub(crate) fn from_sim(file: FileWrapper) -> Self {
        Self(FileVariant::SimulationFile(file))
    }

    pub(crate) fn from_real(file: RealFile) -> Self {
        Self(FileVariant::RealFile(file))
    }

    /// Read into the specified buffer from the specified offset.
    ///
    /// On success, the number of read bytes is returned.
    pub async fn read<'a>(&'a mut self, offset: u64, buf: &'a mut [u8]) -> FsResult<u64> {
        match &mut self.0 {
            FileVariant::SimulationFile(file) => file.read(offset, buf).await,
            FileVariant::RealFile(file) => {
                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(|_| FsError::Unavailable)?;
                file.read(buf)
                    .await
                    .map_err(|_| FsError::Unavailable)
                    .map(|bytes| u64::try_from(bytes).unwrap())
            }
        }
    }

    /// Append passed data to the file.
    ///
    /// On success, the number of appended bytes is returned.
    pub async fn append<'a>(&'a mut self, data: &'a [u8]) -> FsResult<u64> {
        match &mut self.0 {
            FileVariant::SimulationFile(file) => file.append(data).await,
            FileVariant::RealFile(file) => {
                file.seek(SeekFrom::End(0)).await.map_err(|e| {
                    eprintln!("seed error: {}", e);
                    FsError::Unavailable
                })?;
                file.write(data)
                    .await
                    .map_err(|e| {
                        eprintln!("write error: {}", e);
                        FsError::Unavailable
                    })
                    .map(|bytes| u64::try_from(bytes).unwrap())
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Represents error types which can appear when [process][crate::Process]
/// interacts with file system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// Resource can not be created, because it already exists.
    AlreadyExists,
    /// Resource not found.
    NotFound,
    /// Storage is unavailable.
    /// Requested operation can be completed or not.
    Unavailable,
    /// Passed buffer size exceeds size limit.
    BufferSizeExceed,
}

impl From<DSLabFsError> for FsError {
    fn from(value: DSLabFsError) -> Self {
        match value {
            DSLabFsError::AlreadyExists => Self::AlreadyExists,
            DSLabFsError::NotFound => Self::NotFound,
            DSLabFsError::Unavailable => Self::Unavailable,
            DSLabFsError::BufferSizeExceed => Self::BufferSizeExceed,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Represents result of the file system operation.
pub type FsResult<T> = Result<T, FsError>;
