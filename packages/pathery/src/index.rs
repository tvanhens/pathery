use crate::index_loader::IndexLoader;
use std::{fs, path::Path};
use tantivy::{directory::MmapDirectory, schema::Field, Index, IndexWriter};

pub struct IndexProvider {
    schema_loader: IndexLoader,
}

impl IndexProvider {
    pub fn lambda_provider() -> Self {
        IndexProvider {
            schema_loader: IndexLoader::create("/opt/pathery-config")
                .expect("Index should be loadable"),
        }
    }

    pub fn load_index(&self, index_id: &str) -> Index {
        let directory_path = format!("/mnt/pathery-data/{index_id}");

        let index = if let Ok(existing_dir) = MmapDirectory::open(&directory_path) {
            Index::open(existing_dir).expect("Index should be openable")
        } else {
            fs::create_dir(&directory_path).expect("Directory should be creatable");
            let schema = self
                .schema_loader
                .schema_for(index_id)
                .expect("Schema should be loadable");
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
