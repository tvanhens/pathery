use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::message::lambda_writer_sender;
use pathery::schema::SchemaProvider;
use pathery::service::index::{index_doc, PostIndexResponse};
use pathery::{json, tokio};

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::init_tracing();

    let client = &lambda_writer_sender().await;
    let schema_loader = &SchemaProvider::lambda().expect("DirSchema loader should create");

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
