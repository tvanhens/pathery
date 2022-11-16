use pathery::index::{IndexProvider, TantivyIndex};
use pathery::lambda::lambda_runtime::{run, service_fn};
use pathery::lambda::sqs;
use pathery::lambda::*;
use pathery::tantivy::{Document, IndexWriter, Term};
use pathery::{json, serde, tokio};
use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(crate = "self::serde")]
pub enum WriterMessageDetail {
    IndexSingleDoc { document: Document },
    DeleteSingleDoc { doc_id: String },
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(crate = "self::serde")]
pub struct WriterMessage {
    pub index_id: String,
    pub detail: WriterMessageDetail,
}

pub fn delete_doc(writer: &IndexWriter, doc_id: &str) {
    let index = writer.index();
    let id_field = index.id_field();

    writer.delete_term(Term::from_field_text(id_field, doc_id));
}

pub fn index_doc(writer: &IndexWriter, doc: Document) {
    let index = writer.index();
    let id_field = index.id_field();
    let doc_id = doc
        .get_first(id_field)
        .and_then(|id| id.as_text())
        .expect("__id field should be present");

    delete_doc(writer, doc_id);
    writer
        .add_document(doc)
        .expect("Adding a document should not error");
}

async fn function_handler(event: sqs::SqsEvent) -> Result<(), sqs::Error> {
    let provider = IndexProvider::lambda_provider();
    let records = event.payload.records;

    let messages = records
        .iter()
        .map(|message| message.body.as_ref().expect("Body should be present"))
        .map(|body| {
            let msg = json::from_str::<WriterMessage>(body.as_str())
                .expect("Message should be deserializable");
            msg
        })
        .collect::<Vec<_>>();

    let mut writers: HashMap<String, IndexWriter> = HashMap::new();

    for message in messages {
        let index_id = message.index_id;
        let writer = writers
            .entry(index_id.to_string())
            .or_insert_with(|| provider.load_index(&index_id).default_writer());
        match message.detail {
            WriterMessageDetail::IndexSingleDoc { document } => index_doc(writer, document),
            WriterMessageDetail::DeleteSingleDoc { doc_id } => delete_doc(writer, &doc_id),
        }
    }

    for (_index_id, writer) in writers.into_iter() {
        let mut writer = writer;
        writer.commit().expect("commit should succeed");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqs::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
