use dslab_async_mp::storage::file::File as DSLabFile;

use crate::storage::StorageResult;

use super::send_future::{SendFuture, Sf};

pub struct FileWrapper {
    pub file: DSLabFile,
}

impl FileWrapper {
    pub fn append<'a>(&'a mut self, data: &'a [u8]) -> Sf<'a, StorageResult<u64>> {
        SendFuture::from_future(async move { self.file.append(data).await })
    }

    pub fn read<'a>(&'a mut self, offset: u64, buf: &'a mut [u8]) -> Sf<'a, StorageResult<u64>> {
        SendFuture::from_future(async move { self.file.read(offset, buf).await })
    }
}

unsafe impl Sync for FileWrapper {}
unsafe impl Send for FileWrapper {}
