use pathery::chrono::{DateTime, Utc};
use pathery::indexer::Indexer;
use pathery::lambda::{http, http::PatheryRequest, tracing, tracing_subscriber};
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

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");
        let doc_id = event.required_path_param("doc_id");
        let mut indexer = Indexer::create(&index_id)?;

        indexer.delete_doc(&doc_id)?;

        http::success(&DeleteIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
