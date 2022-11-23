use pathery::lambda;
use pathery::lambda::http::HttpRequest;
use pathery::schema::SchemaProvider;
use pathery::service::index::post_index;
use pathery::worker::index_writer::client::IndexWriterClient;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let writer_client = IndexWriterClient::default().await;
    let schema_loader = SchemaProvider::lambda();

    lambda_http::run(lambda_http::service_fn(|event: HttpRequest| {
        post_index(&writer_client, &schema_loader, event.into())
    }))
    .await
}
