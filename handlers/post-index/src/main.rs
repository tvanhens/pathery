use std::time::SystemTime;

use chrono::{DateTime, Utc};
use lambda_http::{run, service_fn, Body, Error, IntoResponse, Request, RequestExt, Response};
use pathery::{index_loader::IndexLoader, indexer::Indexer, lambda};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
struct PostIndexResponse {
    updated_at: String,
}

impl PostIndexResponse {
    fn new() -> PostIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        PostIndexResponse {
            updated_at: now.to_rfc3339(),
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

    let payload = match event.payload::<Value>() {
        Ok(Some(value)) => value,
        _ => {
            return Ok(json!({
                "message": "Invalid body payload"
            })
            .into_response()
            .await);
        }
    };

    let client = lambda::ddb_client().await;

    let mut indexer = Indexer::create(&client, &IndexLoader::lambda().unwrap(), &index_id)?;

    indexer.index_doc(payload)?;

    Ok(serde_json::to_value(PostIndexResponse::new())?
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
