use std::fs;
use std::path::Path;

use tantivy::merge_policy::DefaultMergePolicy;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter};

use crate::directory::PatheryDirectory;
use crate::schema::{SchemaLoader, SchemaProvider};

pub trait IndexLoader: Send + Sync {
    fn load_index(&self, index_id: &str, with_partition: Option<(usize, usize)>) -> Index;
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
    fn load_index(&self, index_id: &str, with_partition: Option<(usize, usize)>) -> Index {
        let directory_path = format!("/mnt/pathery-data/{index_id}");

        let mut index =
            if let Ok(existing_dir) = PatheryDirectory::open(&directory_path, with_partition) {
                Index::open(existing_dir).expect("Index should be openable")
            } else {
                fs::create_dir(&directory_path).expect("Directory should be creatable");
                let schema = self.schema_loader.load_schema(index_id);
                Index::create_in_dir(Path::new(&directory_path), schema)
                    .expect("Index should be creatable")
            };

        index
            .set_default_multithread_executor()
            .expect("default multithread executor should succeed");

        index
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

        let mut merge_policy = DefaultMergePolicy::default();
        merge_policy.set_max_docs_before_merge(10_000);

        writer.set_merge_policy(Box::new(merge_policy));

        writer
    }

    fn id_field(&self) -> Field {
        self.schema()
            .get_field("__id")
            .expect("__id field should exist")
    }
}

#[cfg(test)]
pub mod test_util {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug)]
    pub struct TestIndexLoader {
        schema_loader: SchemaProvider,

        table: Arc<Mutex<HashMap<String, Index>>>,
    }

    impl Clone for TestIndexLoader {
        fn clone(&self) -> Self {
            Self {
                schema_loader: self.schema_loader.clone(),
                table: self.table.clone(),
            }
        }
    }

    impl IndexLoader for TestIndexLoader {
        fn load_index(&self, index_id: &str, _with_partition: Option<(usize, usize)>) -> Index {
            let mut table = self.table.lock().unwrap();

            let entry = (*table).entry(index_id.into());

            let schema = self.schema_loader.load_schema(index_id);

            let index = entry.or_insert_with(|| Index::create_in_ram(schema));

            index.clone()
        }
    }

    impl TestIndexLoader {
        pub fn create(schema_loader: SchemaProvider) -> Self {
            TestIndexLoader {
                schema_loader,
                table: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }
}
