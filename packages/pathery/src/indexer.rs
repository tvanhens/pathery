use anyhow::{anyhow, Result};
use aws_sdk_dynamodb::Client as DDBClient;
use serde_json::Value;
use std::sync::Arc;
use tantivy::{schema::Field, Document, Index, IndexWriter};

use crate::{directory::IndexerDirectory, index_loader::IndexLoader};

pub struct Indexer {
    writer: IndexWriter,
}

impl Indexer {
    pub fn create(
        client: &Arc<DDBClient>,
        index_loader: &IndexLoader,
        index_id: &str,
    ) -> Result<Indexer> {
        tokio::task::block_in_place(|| {
            let directory = IndexerDirectory::create(client, index_id);

            println!("BEFORE_INDEX");
            let index =
                Index::open_or_create(directory, index_loader.schema_for(index_id).unwrap())?;
            println!("AFTER_INDEX");

            Ok(Indexer {
                writer: index.writer(100_000_000)?,
            })
        })
    }

    pub fn index_doc(&mut self, raw_doc: serde_json::Value) -> Result<()> {
        let mut index_doc = Document::new();

        let doc_obj = raw_doc
            .as_object()
            .ok_or_else(|| anyhow!("Expected JSON object"))?;

        for (key, value) in doc_obj {
            let field = self.get_field(key)?;
            match value {
                Value::String(v) => {
                    index_doc.add_text(field, v);
                    Ok(())
                }
                _ => Err(anyhow!("Unrecognized value: {:?}", value)),
            }?;
        }

        self.writer.add_document(index_doc)?;

        self.writer.commit()?;

        Ok(())
    }

    fn get_field(&self, name: &str) -> Result<Field> {
        let schema = self.writer.index().schema();
        let field = schema
            .get_field(name)
            .ok_or_else(|| anyhow!("Field does not exist: {name}"))?;
        Ok(field)
    }
}
