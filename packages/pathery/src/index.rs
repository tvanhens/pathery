use crate::schema::{DirSchemaLoader, SchemaLoader};
use std::{fs, path::Path};
use tantivy::{directory::MmapDirectory, schema::Field, Index, IndexWriter};

pub trait IndexLoader {
    fn load_index(&self, index_id: &str) -> Index;
}

pub struct LambdaIndexProvider {
    schema_loader: DirSchemaLoader,
}

impl LambdaIndexProvider {
    pub fn create() -> Self {
        Self {
            schema_loader: DirSchemaLoader::create().expect("SchemaLoader should create"),
        }
    }
}

impl IndexLoader for LambdaIndexProvider {
    fn load_index(&self, index_id: &str) -> Index {
        let directory_path = format!("/mnt/pathery-data/{index_id}");

        let index = if let Ok(existing_dir) = MmapDirectory::open(&directory_path) {
            Index::open(existing_dir).expect("Index should be openable")
        } else {
            fs::create_dir(&directory_path).expect("Directory should be creatable");
            let schema = self.schema_loader.load_schema(index_id);
            Index::create_in_dir(Path::new(&directory_path), schema)
                .expect("Index should be creatable")
        };

        index
    }
}

pub trait TantivyIndex {
    fn default_writer(&self) -> IndexWriter;

    fn id_field(&self) -> Field;
}

impl TantivyIndex for Index {
    fn default_writer(&self) -> IndexWriter {
        self.writer(100_000_000).expect("Writer should be availble")
    }

    fn id_field(&self) -> Field {
        self.schema()
            .get_field("__id")
            .expect("__id field should exist")
    }
}
