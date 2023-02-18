use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tantivy::directory::error::OpenDirectoryError;
use tantivy::directory::{DirectoryLock, MmapDirectory};
use tantivy::Directory;
use tokio::runtime::Handle;

use crate::pagination::SegmentMeta;
use crate::worker::async_delete::client::AsyncDeleteClient;
use crate::worker::async_delete::job::AsyncDeleteJob;

struct NoopLockGuard;

/// Directory that wraps MmapDirectory without using a lockfile.
///
/// Using a FIFO SQS queue for orchestrating indexing removes the need for a lockfile.
#[derive(Clone, Debug)]
pub struct PatheryDirectory {
    directory_path: PathBuf,

    segments: Option<Vec<SegmentMeta>>,

    inner: MmapDirectory,

    async_delete_client: Arc<dyn AsyncDeleteClient>,

    handle: Handle,
}

impl PatheryDirectory {
    pub fn open<P>(
        directory_path: P,
        async_delete_client: &Arc<dyn AsyncDeleteClient>,
        segments: Option<Vec<SegmentMeta>>,
    ) -> Result<PatheryDirectory, OpenDirectoryError>
    where
        P: AsRef<Path>,
    {
        Ok(PatheryDirectory {
            directory_path: directory_path.as_ref().to_owned(),
            segments,
            inner: MmapDirectory::open(directory_path)?,
            async_delete_client: Arc::clone(async_delete_client),
            handle: Handle::try_current().unwrap(),
        })
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
        let path = self.directory_path.join(path.to_path_buf());
        let job = AsyncDeleteJob::fs_delete(path);
        self.handle
            .block_on(self.async_delete_client.submit_job(job))
            .expect("Message should queue successfully");
        Ok(())
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
        let result = self.inner.atomic_read(path)?;

        // check that we are returning meta.json
        if path == Path::new("meta.json") {
            if let Some(segments) = &self.segments {
                let mut meta: HashMap<String, serde_json::Value> =
                    serde_json::from_slice(&result[..]).expect("meta.json should be parsable");

                // let segments = meta
                //     .get("segments")
                //     .and_then(|s| s.as_array())
                //     .expect("segments should be set");

                // let filtered_segments: Vec<_> = segments
                //     .iter()
                //     .enumerate()
                //     .filter(|(idx, _)| (idx + self.partition_n) % self.total_partitions == 0)
                //     .map(|(_, v)| v.to_owned())
                //     .collect();
                let segments = serde_json::to_value(segments).unwrap();

                meta.insert(String::from("segments"), segments);

                return Ok(serde_json::to_vec(&meta).expect("meta.json should serialize"));
            }
        }

        Ok(result)
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
        Ok(DirectoryLock::from(Box::new(NoopLockGuard)))
    }
}
