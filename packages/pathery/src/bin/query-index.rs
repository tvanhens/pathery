use pathery::index::IndexProvider;
use pathery::lambda;
use pathery::service::index::query_index;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let index_loader = IndexProvider::lambda();

    lambda_http::run(lambda_http::service_fn(|event| {
        query_index(&index_loader, event)
    }))
    .await
}