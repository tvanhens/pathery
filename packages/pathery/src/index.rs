use std::fs;
use std::path::Path;
use std::sync::Arc;

use tantivy::merge_policy::NoMergePolicy;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter};

use crate::directory::PatheryDirectory;
use crate::schema::{SchemaLoader, SchemaProvider};

pub trait IndexLoader: Send + Sync {
    fn load_index(&self, index_id: &str, with_partition: Option<(usize, usize)>) -> Arc<Index>;
}

pub struct IndexProvider {
    schema_loader: SchemaProvider,
}

impl IndexProvider {
    pub fn lambda() -> Self {
        Self {
            schema_loader: SchemaProvider::lambda(),
        }
    }
}

impl IndexLoader for IndexProvider {
    fn load_index(&self, index_id: &str, with_partition: Option<(usize, usize)>) -> Arc<Index> {
        let directory_path = format!("/mnt/pathery-data/{index_id}");

        let index =
            if let Ok(existing_dir) = PatheryDirectory::open(&directory_path, with_partition) {
                Index::open(existing_dir).expect("Index should be openable")
            } else {
                fs::create_dir(&directory_path).expect("Directory should be creatable");
                let schema = self.schema_loader.load_schema(index_id);
                Index::create_in_dir(Path::new(&directory_path), schema)
                    .expect("Index should be creatable")
            };

        Arc::new(index)
    }
}

/// Used for testing purposes. Always returns the same Rc wrapped index.
impl IndexLoader for Arc<Index> {
    fn load_index(&self, _index_id: &str, _with_partition: Option<(usize, usize)>) -> Arc<Index> {
        Arc::clone(self)
    }
}

pub trait IndexExt {
    fn default_writer(&self) -> IndexWriter;

    fn id_field(&self) -> Field;
}

impl IndexExt for Index {
    fn default_writer(&self) -> IndexWriter {
        let writer = self
            .writer(100_000_000)
            .expect("Writer should be available");

        let merge_policy = NoMergePolicy;
        writer.set_merge_policy(Box::new(merge_policy));

        writer
    }

    fn id_field(&self) -> Field {
        self.schema()
            .get_field("__id")
            .expect("__id field should exist")
    }
}
