use std::collections::HashMap;
use std::path::Path;

use tantivy::directory::error::OpenDirectoryError;
use tantivy::directory::{DirectoryLock, MmapDirectory};
use tantivy::Directory;

struct NoopLockGuard;

/// Directory that wraps MmapDirectory without using a lockfile.
///
/// Using a FIFO SQS queue for orchestrating indexing removes the need for a lockfile.
#[derive(Clone, Debug)]
pub struct PatheryDirectory {
    partition_n: usize,

    total_partitions: usize,

    inner: MmapDirectory,
}

impl PatheryDirectory {
    pub fn open<P>(
        directory_path: P,
        with_partition: Option<(usize, usize)>,
    ) -> Result<PatheryDirectory, OpenDirectoryError>
    where
        P: AsRef<Path>,
    {
        let inner = MmapDirectory::open(directory_path)?;
        Ok(PatheryDirectory {
            partition_n: with_partition.map(|x| x.0).unwrap_or(0),
            total_partitions: with_partition.map(|x| x.1).unwrap_or(1),
            inner,
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
        let result = self.inner.atomic_read(path)?;

        // check that we are returning meta.json
        if path == Path::new("meta.json") {
            let mut meta: HashMap<String, serde_json::Value> =
                serde_json::from_slice(&result[..]).expect("meta.json should be parsable");

            let segments = meta
                .get("segments")
                .and_then(|s| s.as_array())
                .expect("segments should be set");

            let filtered_segments: Vec<_> = segments
                .iter()
                .enumerate()
                .filter(|(idx, _)| (idx + self.partition_n) % self.total_partitions == 0)
                .map(|(_, v)| v.to_owned())
                .collect();

            meta.insert(
                String::from("segments"),
                serde_json::Value::Array(filtered_segments),
            );

            Ok(serde_json::to_vec(&meta).expect("meta.json should serialize"))
        } else {
            Ok(result)
        }
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
