use lambda_http::{run, service_fn, Body, Error, IntoResponse, Request, RequestExt, Response};
use pathery::{index_loader::IndexLoader, lambda, searcher::Searcher};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct QueryRequest {
    query: String,
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

    let payload = match event.payload::<QueryRequest>() {
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
    let searcher = Searcher::create(&client, &IndexLoader::lambda()?, &index_id)?;
    let results = searcher.search(&payload.query)?;

    Ok(serde_json::to_value(results)?.into_response().await)
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
