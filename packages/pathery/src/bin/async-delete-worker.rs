use pathery::lambda;
use pathery::lambda::lambda_runtime::{run, service_fn};
use pathery::lambda::sqs;
use pathery::worker::async_delete::handle_event;

#[tokio::main]
async fn main() -> Result<(), sqs::Error> {
    lambda::init_tracing();

    run(service_fn(|event| handle_event(event))).await
}
