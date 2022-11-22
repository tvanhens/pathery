use serde::{Deserialize, Serialize};
use tantivy::Document;

#[derive(Serialize, Deserialize, Debug)]
pub enum IndexWriterOp {
    IndexDoc { document: Document },
    DeleteDoc { doc_id: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpBatch {
    pub index_id: String,
    pub ops: Vec<IndexWriterOp>,
}

impl OpBatch {
    pub fn index_doc(&mut self, document: Document) {
        self.ops.push(IndexWriterOp::IndexDoc { document })
    }

    pub fn delete_doc(&mut self, doc_id: &str) {
        self.ops.push(IndexWriterOp::DeleteDoc {
            doc_id: doc_id.into(),
        })
    }
}
