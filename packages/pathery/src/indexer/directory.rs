use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use tantivy::{
    directory::{DirectoryLock, RamDirectory},
    Directory,
};

use super::filestore::{DynamoFileStore, FileStore};

#[derive(Clone, Debug)]
pub struct IndexerDirectory {
    inner: RamDirectory,
    store: Arc<DynamoFileStore>,
    staged_files: Arc<Mutex<Vec<String>>>,
}

impl IndexerDirectory {
    pub fn create(store_id: &str) -> IndexerDirectory {
        let table_name = std::env::var("TABLE_NAME").unwrap();

        IndexerDirectory {
            inner: RamDirectory::create(),
            staged_files: Arc::new(Mutex::new(Vec::new())),
            store: Arc::new(DynamoFileStore::create(&table_name, store_id)),
        }
    }
}

impl Directory for IndexerDirectory {
    fn get_file_handle(
        &self,
        path: &std::path::Path,
    ) -> Result<Box<dyn tantivy::directory::FileHandle>, tantivy::directory::error::OpenReadError>
    {
        println!("get_file_handle: {}", path.to_str().unwrap());
        self.inner.get_file_handle(path)
    }

    fn delete(&self, path: &std::path::Path) -> Result<(), tantivy::directory::error::DeleteError> {
        println!("delete: {}", path.to_str().unwrap());
        self.inner.delete(path)
    }

    fn exists(
        &self,
        path: &std::path::Path,
    ) -> Result<bool, tantivy::directory::error::OpenReadError> {
        println!("exists: {}", path.to_str().unwrap());
        self.inner.exists(path)
    }

    fn open_write(
        &self,
        path: &std::path::Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        println!("open_write: {}", path.to_str().unwrap());
        let result = self.inner.open_write(path);
        self.staged_files
            .lock()
            .unwrap()
            .push(path.to_str().unwrap().to_owned());
        result
    }

    fn atomic_read(
        &self,
        path: &std::path::Path,
    ) -> Result<Vec<u8>, tantivy::directory::error::OpenReadError> {
        println!("atomic_read: {}", path.to_str().unwrap());
        self.inner.atomic_read(path)
    }

    fn atomic_write(&self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
        println!("atomic_write: {}", path.to_str().unwrap());
        self.staged_files
            .lock()
            .unwrap()
            .push(path.to_str().unwrap().to_owned());
        self.inner.atomic_write(path, data)
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        println!("sync_directory");
        let mut staged_files = self.staged_files.lock().unwrap();
        for path in &*staged_files {
            let content = self.inner.atomic_read(Path::new(&path)).unwrap();
            self.store.write_file(&path, &content).unwrap();
        }
        staged_files.clear();
        self.inner.sync_directory()
    }

    fn watch(
        &self,
        watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        println!("watch");
        self.inner.watch(watch_callback)
    }

    fn acquire_lock(
        &self,
        _lock: &tantivy::directory::Lock,
    ) -> Result<tantivy::directory::DirectoryLock, tantivy::directory::error::LockError> {
        struct SimpleLockGuard {}
        Ok(DirectoryLock::from(Box::new(SimpleLockGuard {})))
    }
}
