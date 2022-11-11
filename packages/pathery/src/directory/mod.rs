pub mod filestore;

use crate::config::AppConfig;
use filestore::{DynamoFileStore, FileStore};
use std::{
    io::{self, BufWriter, Cursor, Seek, SeekFrom, Write},
    sync::Arc,
};
use tantivy::{
    directory::{DirectoryLock, OwnedBytes, WatchHandle},
    Directory,
};
use tantivy_common::{AntiCallToken, TerminatingWrite};

struct StoreWriter {
    path: String,
    store: Arc<DynamoFileStore>,
    data: Cursor<Vec<u8>>,
}

impl StoreWriter {
    fn new(store: Arc<DynamoFileStore>, path: &str) -> StoreWriter {
        StoreWriter {
            data: Cursor::new(Vec::new()),
            store,
            path: path.to_string(),
        }
    }
}

impl Seek for StoreWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.data.seek(pos)
    }
}

impl Write for StoreWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for StoreWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> io::Result<()> {
        self.flush().unwrap();
        self.store
            .write_file(&self.path, self.data.get_ref())
            .unwrap();
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct IndexerDirectory {
    store: Arc<DynamoFileStore>,
}

impl IndexerDirectory {
    pub fn create(store_id: &str) -> IndexerDirectory {
        let config = AppConfig::load();
        let store = Arc::new(DynamoFileStore::create(&config.table_name(), store_id));

        IndexerDirectory { store }
    }
}

impl<'a> Directory for IndexerDirectory {
    fn get_file_handle(
        &self,
        path: &std::path::Path,
    ) -> Result<Box<dyn tantivy::directory::FileHandle>, tantivy::directory::error::OpenReadError>
    {
        let content = self.store.get_content(path.to_str().unwrap()).unwrap();
        Ok(Box::new(OwnedBytes::new(content)))
    }

    fn delete(&self, path: &std::path::Path) -> Result<(), tantivy::directory::error::DeleteError> {
        self.store.delete(path.to_str().unwrap()).unwrap();
        Ok(())
    }

    fn exists(
        &self,
        path: &std::path::Path,
    ) -> Result<bool, tantivy::directory::error::OpenReadError> {
        Ok(self.store.exists(path.to_str().unwrap()).unwrap())
    }

    fn open_write(
        &self,
        path: &std::path::Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        Ok(BufWriter::new(Box::new(StoreWriter::new(
            self.store.clone(),
            path.to_str().unwrap(),
        ))))
    }

    fn atomic_read(
        &self,
        path: &std::path::Path,
    ) -> Result<Vec<u8>, tantivy::directory::error::OpenReadError> {
        if !self.exists(path)? {
            Err(tantivy::directory::error::OpenReadError::FileDoesNotExist(
                path.to_path_buf(),
            ))
        } else {
            let content = self.store.get_content(path.to_str().unwrap()).unwrap();
            Ok(content)
        }
    }

    fn atomic_write(&self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
        self.store
            .write_file(path.to_str().unwrap(), &data.to_vec())
            .unwrap();
        Ok(())
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn watch(
        &self,
        watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        Ok(WatchHandle::new(Arc::new(watch_callback)))
    }

    fn acquire_lock(
        &self,
        _lock: &tantivy::directory::Lock,
    ) -> Result<tantivy::directory::DirectoryLock, tantivy::directory::error::LockError> {
        struct SimpleLockGuard {}
        Ok(DirectoryLock::from(Box::new(SimpleLockGuard {})))
    }
}
