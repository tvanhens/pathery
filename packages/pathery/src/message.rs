use serde;
use tantivy::Document;

#[derive(serde::Deserialize, serde::Serialize)]
pub enum WriterMessageDetail {
    IndexSingleDoc { document: Document },
    DeleteSingleDoc { doc_id: String },
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct WriterMessage {
    pub index_id: String,
    pub detail: WriterMessageDetail,
}

impl WriterMessage {
    pub fn index_single_doc(index_id: &str, document: Document) -> Self {
        WriterMessage {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::IndexSingleDoc { document },
        }
    }

    pub fn delete_doc(index_id: &str, doc_id: &str) -> Self {
        WriterMessage {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::DeleteSingleDoc {
                doc_id: doc_id.to_string(),
            },
        }
    }
}
