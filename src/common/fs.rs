//! Manipulations with files.

use std::io::SeekFrom;

use async_std::{
    fs::File as RealFile,
    io::{prelude::SeekExt, ReadExt, WriteExt},
};
use dslab_async_mp::storage::result::StorageError;

use crate::{sim::fs::FileWrapper, storage::StorageResult};

////////////////////////////////////////////////////////////////////////////////

enum FileVariant {
    SimulationFile(FileWrapper),
    RealFile(RealFile),
}

/// Abstraction over the file. Process can create, open, read and write to file.
/// To [create][crate::Context::create_file] or [open][crate::Context::open_file] the file,
/// refer to the corresponding context methods.
pub struct File(FileVariant);

impl File {
    pub(crate) fn from_sim(file: FileWrapper) -> Self {
        Self(FileVariant::SimulationFile(file))
    }

    pub(crate) fn from_real(file: RealFile) -> Self {
        Self(FileVariant::RealFile(file))
    }

    /// Read into the specified buffer from the specified offset.
    /// On success, the number of read bytes is returned.
    pub async fn read<'a>(&'a mut self, offset: u64, buf: &'a mut [u8]) -> StorageResult<u64> {
        match &mut self.0 {
            FileVariant::SimulationFile(file) => file.read(offset, buf).await,
            FileVariant::RealFile(file) => {
                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(|_| StorageError::Unavailable)?;
                file.read(buf)
                    .await
                    .map_err(|_| StorageError::Unavailable)
                    .map(|bytes| u64::try_from(bytes).unwrap())
            }
        }
    }

    /// Append passed data to the file.
    /// On success, the number of appended bytes is returned.
    pub async fn append<'a>(&'a mut self, data: &'a [u8]) -> StorageResult<u64> {
        match &mut self.0 {
            FileVariant::SimulationFile(file) => file.append(data).await,
            FileVariant::RealFile(file) => {
                file.seek(SeekFrom::End(0)).await.map_err(|e| {
                    eprintln!("seed error: {}", e);
                    StorageError::Unavailable
                })?;
                file.write(data)
                    .await
                    .map_err(|e| {
                        eprintln!("write error: {}", e);
                        StorageError::Unavailable
                    })
                    .map(|bytes| u64::try_from(bytes).unwrap())
            }
        }
    }
}
