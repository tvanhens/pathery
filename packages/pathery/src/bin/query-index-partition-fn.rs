use pathery::function::query_index_partition::handle_event;
use pathery::index::LambdaIndexLoader;
use pathery::lambda;
use pathery::lambda::lambda_runtime::{run, service_fn};
use pathery::lambda::sqs;

#[tokio::main]
async fn main() -> Result<(), sqs::Error> {
    lambda::init_tracing();

    let index_loader = LambdaIndexLoader::create().await;

    run(service_fn(|event| handle_event(&index_loader, event))).await
}
