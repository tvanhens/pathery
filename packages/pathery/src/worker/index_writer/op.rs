use serde::{Deserialize, Serialize};
use tantivy::schema::Schema;
use tantivy::Document;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum IndexWriterOp {
    IndexDoc { document: String },
    DeleteDoc { doc_id: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OpBatch {
    pub index_id: String,
    pub ops: Vec<IndexWriterOp>,
}

impl OpBatch {
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
}
