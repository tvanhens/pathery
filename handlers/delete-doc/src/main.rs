use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use pathery::chrono::{DateTime, Utc};
use pathery::lambda::{self, http, tracing, tracing_subscriber, IntoResponse, RequestExt};
use pathery::{index_loader::IndexLoader, indexer::Indexer};
use pathery::{json, serde, tokio};

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

    let client = &lambda::ddb_client().await;

    let handler = move |event: http::Request| -> Pin<
        Box<dyn Future<Output = Result<http::Response<http::Body>, http::Error>> + Send>,
    > {
        Box::pin(async move {
            let index_id = {
                let params = event.path_parameters();
                if let Some(index_id) = params.first("index_id") {
                    index_id.to_string()
                } else {
                    return Ok(json::json!({
                        "message": "Missing path_param index_id"
                    })
                    .into_response()
                    .await);
                }
            };

            let doc_id = {
                let params = event.path_parameters();
                if let Some(index_id) = params.first("doc_id") {
                    index_id.to_string()
                } else {
                    return Ok(json::json!({
                        "message": "Missing path_param doc_id"
                    })
                    .into_response()
                    .await);
                }
            };

            let mut indexer = Indexer::create(client, &IndexLoader::lambda().unwrap(), &index_id)?;

            indexer.delete_doc(&doc_id)?;

            Ok(json::to_value(DeleteIndexResponse::new(&doc_id))?
                .into_response()
                .await)
        })
    };

    http::run(http::service_fn(handler)).await
}
