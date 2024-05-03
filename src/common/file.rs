//! Definition of [`File`], which allows user to work with file open file on the disk.

use std::io::SeekFrom;

use async_std::{
    fs::File as RealFile,
    io::{prelude::SeekExt, ReadExt, WriteExt},
};
use dslab_async_mp::storage::result::StorageError;

use crate::{storage::StorageResult, virt::file::FileWrapper};

pub enum File {
    SimulationFile(FileWrapper),
    RealFile(RealFile),
}

impl File {
    /// Read into the specified buffer from the specified offset.
    pub async fn read<'a>(&'a mut self, offset: u64, buf: &'a mut [u8]) -> StorageResult<u64> {
        match self {
            File::SimulationFile(file) => file.read(offset, buf).await,
            File::RealFile(file) => {
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
    pub async fn append<'a>(&'a mut self, data: &'a [u8]) -> StorageResult<u64> {
        match self {
            File::SimulationFile(file) => file.append(data).await,
            File::RealFile(file) => {
                file.seek(SeekFrom::End(0))
                    .await
                    .map_err(|_| StorageError::Unavailable)?;
                file.write(data)
                    .await
                    .map_err(|_| StorageError::Unavailable)
                    .map(|bytes| u64::try_from(bytes).unwrap())
            }
        }
    }
}
