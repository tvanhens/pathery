use std::time::SystemTime;

use chrono::{DateTime, Utc};
use lambda_http::{run, service_fn, Body, Error, IntoResponse, Request, RequestExt, Response};
use pathery::{index_loader::IndexLoader, indexer::Indexer, lambda};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
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

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let index_id = {
        let params = event.path_parameters();
        if let Some(index_id) = params.first("index_id") {
            index_id.to_string()
        } else {
            return Ok(json!({
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
            return Ok(json!({
                "message": "Missing path_param doc_id"
            })
            .into_response()
            .await);
        }
    };

    let client = lambda::ddb_client().await;

    let mut indexer = Indexer::create(&client, &IndexLoader::lambda().unwrap(), &index_id)?;

    indexer.delete_doc(&doc_id)?;

    Ok(serde_json::to_value(DeleteIndexResponse::new(&doc_id))?
        .into_response()
        .await)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
