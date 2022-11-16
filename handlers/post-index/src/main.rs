use pathery::aws::lambda_queue_client;
use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::schema::DirSchemaLoader;
use pathery::{json, tokio};
use post_index::{index_doc, PostIndexResponse};

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::tracing_subscriber::fmt()
        .with_max_level(lambda::tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let client = &lambda_queue_client().await;
    let schema_loader = &DirSchemaLoader::create().expect("DirSchema loader should create");

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<json::Value>() {
            Ok(v) => v,
            Err(err) => return err.into(),
        };

        let doc_id = match index_doc(client, schema_loader, &index_id, &payload).await {
            Ok(v) => v,
            Err(err) => return err.into(),
        };

        http::success(&PostIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
