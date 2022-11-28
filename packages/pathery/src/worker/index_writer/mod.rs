pub mod client;
pub mod op;

use std::collections::HashMap;

use serde_json as json;
use tantivy::{Document, IndexWriter, Term};

use self::op::{IndexWriterOp, OpBatch};
use crate::aws::{S3Bucket, S3Ref};
use crate::index::{IndexLoader, TantivyIndex};
use crate::lambda::{self, sqs};

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

pub async fn handle_event(
    bucket_client: &dyn S3Bucket<OpBatch>,
    index_loader: &dyn IndexLoader,
    event: sqs::SqsEvent,
) -> Result<(), lambda::Error> {
    let records = event.payload.records;

    let messages = records
        .iter()
        .map(|message| message.body.as_ref().expect("Body should be present"))
        .map(|body| {
            let msg =
                json::from_str::<S3Ref>(body.as_str()).expect("Message should be deserializable");
            msg
        })
        .collect::<Vec<_>>();

    let mut writers: HashMap<String, IndexWriter> = HashMap::new();

    for s3_ref in messages {
        let batch = bucket_client
            .read_object(&s3_ref)
            .await
            .expect("batch should load");
        let index_id = batch.index_id;
        let writer = writers
            .entry(index_id.to_string())
            .or_insert_with(|| index_loader.load_index(&index_id).default_writer());

        let schema = writer.index().schema();

        for op in batch.ops {
            match op {
                IndexWriterOp::IndexDoc { document } => {
                    index_doc(writer, schema.parse_document(&document).unwrap())
                }
                IndexWriterOp::DeleteDoc { doc_id } => delete_doc(writer, &doc_id),
            }
        }

        writer.commit().expect("commit should succeed");
        tracing::info!(message = "index_committed", index_id);

        bucket_client.delete_object(&s3_ref).await;
    }

    for (_index_id, writer) in writers.into_iter() {
        writer
            .wait_merging_threads()
            .expect("merge should finish without error");
    }

    Ok(())
}

pub fn batch(index_id: &str) -> OpBatch {
    OpBatch {
        index_id: index_id.into(),
        ops: Vec::new(),
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use async_trait::async_trait;
    use aws_lambda_events::sqs::{self, SqsMessage};
    use lambda_http::Context;
    use lambda_runtime::LambdaEvent;
    use serde_json as json;
    use tantivy::Index;

    use super::op::OpBatch;
    use super::{batch, handle_event};
    use crate::aws::{S3Bucket, S3Ref};
    use crate::schema::{SchemaExt, SchemaLoader, SchemaProvider};

    pub fn setup() -> Arc<Index> {
        let config = json::json!({
            "indexes": [
                {
                    "prefix": "test",
                    "fields": [
                        {
                            "name": "year",
                            "kind": "i64",
                            "flags": ["INDEXED", "STORED"]
                        }
                    ]
                }
            ]
        });

        let schema_provider = SchemaProvider::from_json(config);

        let index = Index::create_in_ram(schema_provider.load_schema("test"));

        Arc::new(index)
    }

    #[async_trait]
    impl S3Bucket<OpBatch> for OpBatch {
        async fn read_object(&self, _s3_ref: &S3Ref) -> Option<OpBatch> {
            let serialized = json::to_string(self).unwrap();
            Some(json::from_str(&serialized).unwrap())
        }

        async fn write_object(&self, _key: &str, _object: &OpBatch) -> Option<S3Ref> {
            todo!()
        }

        async fn delete_object(&self, _s3_ref: &S3Ref) {}
    }

    #[tokio::test]
    async fn test_indexing() {
        let index_provider = setup();
        let mut op_batch = batch("test");

        let (_, document) = index_provider
            .schema()
            .to_document(json::json!({
                "year": 1989
            }))
            .unwrap();

        op_batch.index_doc(&index_provider.schema(), document);

        let message = SqsMessage {
            body: Some(
                json::to_string(&S3Ref {
                    bucket: String::from(""),
                    key: String::from(""),
                })
                .unwrap(),
            ),
            ..Default::default()
        };

        let event = sqs::SqsEvent {
            records: vec![message],
        };

        handle_event(
            &op_batch,
            &index_provider,
            LambdaEvent::new(event, Context::default()),
        )
        .await
        .unwrap();
    }
}
