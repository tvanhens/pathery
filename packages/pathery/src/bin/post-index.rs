use pathery::lambda::{self, http};
use pathery::message::lambda_writer_sender;
use pathery::schema::SchemaProvider;
use pathery::service::index::post_index;

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::init_tracing();

    let writer_client = lambda_writer_sender().await;
    let schema_loader = SchemaProvider::lambda().expect("DirSchema loader should create");

    http::run(http::service_fn(|event| {
        post_index(&writer_client, &schema_loader, event)
    }))
    .await
}
