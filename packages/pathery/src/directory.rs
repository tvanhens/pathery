use std::path::Path;

use tantivy::directory::error::OpenDirectoryError;
use tantivy::directory::{DirectoryLock, MmapDirectory};
use tantivy::Directory;

/// Directory that wraps MmapDirectory without using a lockfile.
///
/// Using a FIFO SQS queue for orchestrating indexing removes the need for a lockfile.
#[derive(Clone, Debug)]
pub struct PatheryDirectory {
    inner: MmapDirectory,
}

impl PatheryDirectory {
    pub fn open<P>(directory_path: P) -> Result<PatheryDirectory, OpenDirectoryError>
    where P: AsRef<Path> {
        let inner = MmapDirectory::open(directory_path)?;
        Ok(PatheryDirectory { inner })
    }
}

impl Directory for PatheryDirectory {
    fn get_file_handle(
        &self,
        path: &std::path::Path,
    ) -> Result<Box<dyn tantivy::directory::FileHandle>, tantivy::directory::error::OpenReadError>
    {
        self.inner.get_file_handle(path)
    }

    fn delete(&self, path: &std::path::Path) -> Result<(), tantivy::directory::error::DeleteError> {
        self.inner.delete(path)
    }

    fn exists(
        &self,
        path: &std::path::Path,
    ) -> Result<bool, tantivy::directory::error::OpenReadError> {
        self.inner.exists(path)
    }

    fn open_write(
        &self,
        path: &std::path::Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        self.inner.open_write(path)
    }

    fn atomic_read(
        &self,
        path: &std::path::Path,
    ) -> Result<Vec<u8>, tantivy::directory::error::OpenReadError> {
        self.inner.atomic_read(path)
    }

    fn atomic_write(&self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
        self.inner.atomic_write(path, data)
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        self.inner.sync_directory()
    }

    fn watch(
        &self,
        watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        self.inner.watch(watch_callback)
    }

    fn acquire_lock(
        &self,
        _lock: &tantivy::directory::Lock,
    ) -> Result<tantivy::directory::DirectoryLock, tantivy::directory::error::LockError> {
        struct NoopLockGuard {}
        Ok(DirectoryLock::from(Box::new(NoopLockGuard {})))
    }
}
