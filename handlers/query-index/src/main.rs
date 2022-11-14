use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
use pathery::{index_loader::IndexLoader, lambda, searcher::Searcher};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize, Deserialize)]
struct QueryResponse {
    results: Vec<String>,
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    if let Body::Text(body) = event.body() {
        let body_safe = body.to_string();
        let path_params = event.path_parameters();

        let index_id = path_params.first("index_id").unwrap();

        let value = serde_json::from_str::<QueryRequest>(&body_safe).unwrap();

        let client = lambda::ddb_client().await;

        let searcher =
            Searcher::create(&client, &IndexLoader::lambda().unwrap(), index_id).unwrap();

        let result = searcher.search(&value.query).unwrap();

        let resp = Response::builder()
            .status(200)
            .header("content-type", "text/html")
            .body(
                serde_json::to_string(&QueryResponse { results: result })
                    .unwrap()
                    .into(),
            )
            .map_err(Box::new)?;
        Ok(resp)
    } else {
        panic!("Expected body text");
    }
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
