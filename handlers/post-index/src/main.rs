use pathery::index::IndexProvider;
use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::{json, tokio};
use post_index::{index_doc, PostIndexResponse};

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

        let index = IndexProvider::lambda_provider().load_index(&index_id);

        let doc_id = match index_doc(&index, &payload) {
            Ok(v) => v,
            Err(err) => return err.into(),
        };

        http::success(&PostIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
