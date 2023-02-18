use std::fs;
use std::path::Path;
use std::sync::Arc;

use tantivy::merge_policy::DefaultMergePolicy;
use tantivy::schema::Field;
use tantivy::{Index, IndexWriter};

use crate::directory::PatheryDirectory;
use crate::pagination::SegmentMeta;
use crate::schema::{SchemaLoader, SchemaProvider};
use crate::service::ServiceError;
use crate::worker::async_delete::client::{AsyncDeleteClient, LambdaAsyncDeleteClient};

pub trait IndexLoader: Send + Sync {
    fn load_index(
        &self,
        index_id: &str,
        segments: Option<Vec<SegmentMeta>>,
    ) -> Result<Index, ServiceError>;
}

pub struct LambdaIndexLoader {
    schema_loader: SchemaProvider,

    async_delete_client: Arc<dyn AsyncDeleteClient>,
}

impl LambdaIndexLoader {
    pub async fn create() -> Self {
        let async_delete_client = LambdaAsyncDeleteClient::create(None).await;
        let async_delete_client = Arc::new(async_delete_client);

        Self {
            schema_loader: SchemaProvider::lambda(),
            async_delete_client,
        }
    }
}

impl IndexLoader for LambdaIndexLoader {
    fn load_index(
        &self,
        index_id: &str,
        segments: Option<Vec<SegmentMeta>>,
    ) -> Result<Index, ServiceError> {
        let directory_path = format!("/mnt/pathery-data/{index_id}");

        let mut index = if let Ok(existing_dir) =
            PatheryDirectory::open(&directory_path, &self.async_delete_client, segments)
        {
            Index::open(existing_dir).expect("Index should be openable")
        } else {
            fs::create_dir(&directory_path).expect("Directory should be creatable");
            let schema = self.schema_loader.load_schema(index_id)?;
            Index::create_in_dir(Path::new(&directory_path), schema)
                .expect("Index should be creatable")
        };

        index
            .set_default_multithread_executor()
            .expect("default multithread executor should succeed");

        Ok(index)
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
        fn load_index(
            &self,
            index_id: &str,
            _segments: Option<Vec<SegmentMeta>>,
        ) -> Result<Index, ServiceError> {
            let mut table = self.table.lock().unwrap();

            let entry = (*table).entry(index_id.into());

            let schema = self.schema_loader.load_schema(index_id)?;

            let index = entry.or_insert_with(|| Index::create_in_ram(schema));

            Ok(index.clone())
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
