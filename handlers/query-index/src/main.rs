use pathery::lambda::{http, http::PatheryRequest};
use pathery::{index_loader::IndexLoader, lambda, searcher::Searcher};
use pathery::{serde, tokio};

#[derive(serde::Deserialize)]
#[serde(crate = "self::serde")]
struct QueryRequest {
    query: String,
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::tracing_subscriber::fmt()
        .with_max_level(lambda::tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let client = &lambda::ddb_client().await;

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<QueryRequest>() {
            Ok(value) => value,
            Err(err) => return err.into(),
        };

        let searcher = Searcher::create(&client, &IndexLoader::lambda()?, &index_id)?;

        let results = searcher.search(&payload.query)?;

        http::success(&results)
    };

    http::run(http::service_fn(handler)).await
}
