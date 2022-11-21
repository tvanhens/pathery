use pathery::index::IndexProvider;
use pathery::lambda::{self, http};
use pathery::service::index::query_index;

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::init_tracing();

    let index_loader = IndexProvider::lambda();

    http::run(http::service_fn(|event| query_index(&index_loader, event))).await
}
