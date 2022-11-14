use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
use pathery::{index_loader::IndexLoader, indexer::Indexer, lambda};

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    if let Body::Text(body) = event.body() {
        let body_safe = body.to_string();
        let path_params = event.path_parameters();

        let index_id = path_params.first("index_id").unwrap();

        let value = serde_json::from_str::<serde_json::Value>(&body_safe).unwrap();

        let client = lambda::ddb_client().await;

        let mut indexer =
            Indexer::create(&client, &IndexLoader::lambda().unwrap(), index_id).unwrap();

        indexer.index_doc(value).unwrap();
    }

    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body("Success".into())
        .map_err(Box::new)?;
    Ok(resp)
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
