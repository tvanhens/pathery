use anyhow::{anyhow, Result};
use serde_json::Value;
use tantivy::{schema::Field, Document, IndexWriter, Term};

use crate::index::IndexProvider;

fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

pub struct Indexer {
    writer: IndexWriter,
}

impl Indexer {
    pub fn create(index_id: &str) -> Result<Indexer> {
        let index = IndexProvider::lambda_provider().load_index(index_id);

        Ok(Indexer {
            writer: index.writer(100_000_000)?,
        })
    }

    pub fn index_doc(&mut self, raw_doc: &serde_json::Value) -> Result<String> {
        let mut doc_obj = raw_doc.clone();
        let doc_obj = doc_obj
            .as_object_mut()
            .ok_or_else(|| anyhow!("Expected JSON object"))?;

        let id = doc_obj
            .remove("__id")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| generate_id());

        let id_field = self.get_field("__id")?;
        let mut index_doc = Document::new();

        index_doc.add_text(id_field, &id);

        for (key, value) in doc_obj.iter() {
            let field = self.get_field(key)?;
            match value {
                Value::String(v) => {
                    index_doc.add_text(field, v);
                    Ok(())
                }
                _ => Err(anyhow!("Unrecognized value: {:?}", value)),
            }?;
        }

        self.writer
            .delete_term(Term::from_field_text(id_field, &id));
        self.writer.add_document(index_doc)?;

        self.writer.commit()?;

        Ok(id)
    }

    pub fn delete_doc(&mut self, doc_id: &str) -> Result<()> {
        let id_field = self.get_field("__id")?;
        self.writer
            .delete_term(Term::from_field_text(id_field, doc_id));
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
