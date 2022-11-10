use lambda_http::{run, service_fn, Body, Error, Request, Response};
use pathery::indexer::Indexer;
use tokio::runtime::Handle;

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    if let Body::Text(body) = event.body() {
        let body_safe = body.to_string();
        Handle::current()
            .spawn_blocking(move || {
                let value = serde_json::from_str::<serde_json::Value>(&body_safe).unwrap();

                let mut indexer = Indexer::create().unwrap();

                indexer.index_doc(value).unwrap();
            })
            .await
            .unwrap();
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