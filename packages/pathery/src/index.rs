use std::{fs, path::Path};

use tantivy::{directory::MmapDirectory, Index};

use crate::index_loader::IndexLoader;

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
