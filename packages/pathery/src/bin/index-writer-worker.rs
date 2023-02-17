use pathery::index::LambdaIndexLoader;
use pathery::lambda;
use pathery::lambda::lambda_runtime::{run, service_fn};
use pathery::lambda::sqs;
use pathery::store::document::DDBDocumentStore;
use pathery::worker::index_writer::handle_event;

#[tokio::main]
async fn main() -> Result<(), sqs::Error> {
    lambda::init_tracing();

    let document_store = DDBDocumentStore::create(None).await;
    let index_loader = LambdaIndexLoader::create().await;

    run(service_fn(|event| {
        handle_event(&document_store, &index_loader, event)
    }))
    .await
}
