use pathery::chrono::{DateTime, Utc};
use pathery::indexer::Indexer;
use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::{json, serde, tokio};
use std::time::SystemTime;

#[derive(serde::Serialize)]
#[serde(crate = "self::serde")]
struct PostIndexResponse {
    #[serde(rename = "__id")]
    doc_id: String,
    updated_at: String,
}

impl PostIndexResponse {
    fn new(doc_id: &str) -> PostIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        PostIndexResponse {
            doc_id: doc_id.to_string(),
            updated_at: now.to_rfc3339(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::tracing_subscriber::fmt()
        .with_max_level(lambda::tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<json::Value>() {
            Ok(v) => v,
            Err(err) => return err.into(),
        };

        let mut indexer = Indexer::create(&index_id)?;

        let doc_id = indexer.index_doc(&payload)?;

        http::success(&PostIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
