pub mod client;
pub mod job;

use std::collections::HashMap;

use serde_json as json;
use tantivy::{Document, IndexWriter, Term};
use tracing::info;

use self::job::{IndexWriterOp, Job};
use crate::index::{IndexExt, IndexLoader};
use crate::lambda::{self, sqs};
use crate::store::document::{DocumentStore, SearchDocRef};

fn delete_doc(writer: &IndexWriter, doc_id: &str) {
    let index = writer.index();
    let id_field = index.id_field();

    writer.delete_term(Term::from_field_text(id_field, doc_id));
    tracing::info!(message = "doc_deleted", doc_id);
}

fn index_doc(writer: &IndexWriter, doc: Document) {
    let index = writer.index();
    let id_field = index.id_field();
    let doc_id = doc
        .get_first(id_field)
        .and_then(|id| id.as_text())
        .expect("__id field should be present")
        .to_string();

    delete_doc(writer, &doc_id);
    writer
        .add_document(doc)
        .expect("Adding a document should not error");
    tracing::info!(message = "doc_indexed", doc_id);
}

pub async fn handle_job(writer: &mut IndexWriter, document_store: &dyn DocumentStore, job: Job) {
    let schema = writer.index().schema();

    let mut doc_refs: Vec<SearchDocRef> = vec![];

    for op in job.ops {
        match op {
            IndexWriterOp::IndexDoc { doc_ref } => doc_refs.push(doc_ref),

            IndexWriterOp::DeleteDoc { doc_id } => delete_doc(writer, doc_id.id()),
        }
    }

    let docs = document_store.get_documents(doc_refs).await.unwrap();

    for doc in docs {
        let document = doc.document(&schema);
        index_doc(writer, document);
    }
}

pub async fn handle_event(
    document_store: &dyn DocumentStore,
    index_loader: &dyn IndexLoader,
    event: sqs::SqsEvent,
) -> Result<(), lambda::Error> {
    let records = event.payload.records;

    let jobs = records
        .iter()
        .map(|message| message.body.as_ref().expect("Body should be present"))
        .map(|body| {
            let msg =
                json::from_str::<Job>(body.as_str()).expect("Message should be deserializable");
            msg
        })
        .collect::<Vec<_>>();

    let mut writers: HashMap<String, IndexWriter> = HashMap::new();

    for job in jobs {
        let index_id = &job.index_id;
        let mut writer = writers.entry(index_id.to_string()).or_insert_with(|| {
            index_loader
                .load_index(&index_id, None)
                .unwrap()
                .default_writer()
        });

        handle_job(&mut writer, document_store, job).await;
    }

    for (index, mut writer) in writers.into_iter() {
        writer.commit().expect("commit should succeed");
        info!(message = "index_commit", index);
        writer
            .wait_merging_threads()
            .expect("merge should finish without error");
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use aws_lambda_events::sqs::{self, SqsMessage};
    use lambda_http::Context;
    use lambda_runtime::LambdaEvent;

    use super::job::Job;
    use super::{handle_event, *};
    use crate::schema::SchemaLoader;
    use crate::search_doc::SearchDoc;
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_indexing() {
        let ctx = setup();

        let schema = ctx.schema_loader().load_schema("test").unwrap();

        let mut job = Job::create("test");

        let document = SearchDoc::from_json(
            &schema,
            json!({
                "year": 1989
            }),
        )
        .unwrap();

        let doc_refs = ctx
            .document_store()
            .save_documents(vec![document])
            .await
            .unwrap();

        for doc_ref in doc_refs {
            job.index_doc(doc_ref);
        }

        let message = SqsMessage {
            body: Some(json::to_string(&job).unwrap()),
            ..Default::default()
        };

        let event = sqs::SqsEvent {
            records: vec![message],
        };

        handle_event(
            ctx.document_store(),
            ctx.index_loader(),
            LambdaEvent::new(event, Context::default()),
        )
        .await
        .unwrap();

        assert_eq!(
            1,
            ctx.index_loader()
                .load_index("test", None)
                .unwrap()
                .reader()
                .unwrap()
                .searcher()
                .num_docs()
        );
    }
}
