use pathery::chrono::{DateTime, Utc};
use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::message::{lambda_writer_sender, WriterMessage, WriterSender};
use pathery::{serde, tokio};
use std::time::SystemTime;

#[derive(serde::Serialize)]
#[serde(crate = "self::serde")]
struct DeleteIndexResponse {
    #[serde(rename = "__id")]
    doc_id: String,
    deleted_at: String,
}

impl DeleteIndexResponse {
    fn new(doc_id: &str) -> DeleteIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        DeleteIndexResponse {
            doc_id: doc_id.to_string(),
            deleted_at: now.to_rfc3339(),
        }
    }
}

async fn delete_doc<C>(client: &C, index_id: &str, doc_id: &str)
where
    C: WriterSender,
{
    client
        .send_message(index_id, &WriterMessage::delete_doc(index_id, doc_id))
        .await;
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::init_tracing();

    let client = &lambda_writer_sender().await;

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");
        let doc_id = event.required_path_param("doc_id");

        delete_doc(client, &index_id, &doc_id).await;

        http::success(&DeleteIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
