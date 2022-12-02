use serde::{Deserialize, Serialize};
use tantivy::schema::Schema;
use tantivy::Document;

use crate::store::document::SearchDocRef;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum IndexWriterOp {
    IndexDoc { document: String },

    IndexBatch { refs: Vec<SearchDocRef> },

    DeleteDoc { doc_id: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Job {
    pub index_id: String,
    pub ops: Vec<IndexWriterOp>,
}

impl Job {
    pub fn create(index_id: &str) -> Job {
        Job {
            index_id: index_id.into(),
            ops: vec![],
        }
    }

    pub fn index_doc(&mut self, schema: &Schema, document: Document) {
        self.ops.push(IndexWriterOp::IndexDoc {
            document: schema.to_json(&document),
        })
    }

    pub fn delete_doc(&mut self, doc_id: &str) {
        self.ops.push(IndexWriterOp::DeleteDoc {
            doc_id: doc_id.into(),
        })
    }

    pub fn index_batch(&mut self, doc_refs: Vec<SearchDocRef>) {
        self.ops.push(IndexWriterOp::IndexBatch { refs: doc_refs });
    }
}
