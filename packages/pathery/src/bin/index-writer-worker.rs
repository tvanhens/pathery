use pathery::aws::s3_bucket_client;
use pathery::index::IndexProvider;
use pathery::lambda;
use pathery::lambda::lambda_runtime::{run, service_fn};
use pathery::lambda::sqs;
use pathery::worker::index_writer::handle_event;

#[tokio::main]
async fn main() -> Result<(), sqs::Error> {
    lambda::init_tracing();

    let bucket_client = s3_bucket_client().await;
    let index_loader = IndexProvider::lambda();

    run(service_fn(|event| {
        handle_event(&bucket_client, &index_loader, event)
    }))
    .await
}
