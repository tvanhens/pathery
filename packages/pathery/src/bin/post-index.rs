use pathery::lambda;
use pathery::message::lambda_writer_sender;
use pathery::schema::SchemaProvider;
use pathery::service::index::post_index;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let writer_client = lambda_writer_sender().await;
    let schema_loader = SchemaProvider::lambda().expect("DirSchema loader should create");

    lambda_http::run(lambda_http::service_fn(|event| {
        post_index(&writer_client, &schema_loader, event)
    }))
    .await
}
