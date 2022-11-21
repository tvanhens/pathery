use serde::{Deserialize, Serialize};
use tantivy::Document;

#[derive(Serialize, Deserialize, Debug)]
pub enum WriterMessageDetail {
    IndexSingleDoc { document: Document },
    DeleteSingleDoc { doc_id: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexWriterOp {
    pub index_id: String,
    pub detail: WriterMessageDetail,
}

impl IndexWriterOp {
    pub fn index_single_doc(index_id: &str, document: Document) -> Self {
        IndexWriterOp {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::IndexSingleDoc { document },
        }
    }

    pub fn delete_doc(index_id: &str, doc_id: &str) -> Self {
        IndexWriterOp {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::DeleteSingleDoc {
                doc_id: doc_id.to_string(),
            },
        }
    }
}
