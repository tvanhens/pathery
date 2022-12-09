use serde::{Deserialize, Serialize};

use crate::search_doc::SearchDocId;
use crate::store::document::SearchDocRef;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum IndexWriterOp {
    IndexDoc { doc_ref: SearchDocRef },

    DeleteDoc { doc_id: SearchDocId },
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

    pub fn index_doc(&mut self, doc_ref: SearchDocRef) {
        self.ops.push(IndexWriterOp::IndexDoc { doc_ref })
    }

    pub fn delete_doc(&mut self, doc_id: SearchDocId) {
        self.ops.push(IndexWriterOp::DeleteDoc { doc_id })
    }
}
