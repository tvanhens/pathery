use pathery::index::IndexProvider;
use pathery::lambda;
use pathery::lambda::http::HttpRequest;
use pathery::service::index::query_index;
use pathery::store::document::DDBDocumentStore;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let document_store = DDBDocumentStore::create(None).await;
    let index_loader = IndexProvider::lambda();

    lambda_http::run(lambda_http::service_fn(|event: HttpRequest| {
        query_index(&document_store, &index_loader, event.into())
    }))
    .await
}
