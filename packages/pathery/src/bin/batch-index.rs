use pathery::lambda;
use pathery::lambda::http::HttpRequest;
use pathery::schema::SchemaProvider;
use pathery::service::index::batch_index;
use pathery::store::document::DDBDocumentStore;
use pathery::worker::index_writer::client::LambdaIndexWriterClient;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let document_store = DDBDocumentStore::create(None).await;
    let writer_client = LambdaIndexWriterClient::create();
    let schema_loader = SchemaProvider::lambda();

    lambda_http::run(lambda_http::service_fn(|event: HttpRequest| {
        batch_index(
            &document_store,
            &writer_client,
            &schema_loader,
            event.into(),
        )
    }))
    .await
}
