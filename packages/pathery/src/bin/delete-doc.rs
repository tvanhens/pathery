use pathery::lambda::{self, http};
use pathery::message::lambda_writer_sender;
use pathery::service::doc::delete_doc;

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::init_tracing();

    let client = lambda_writer_sender().await;

    http::run(http::service_fn(|event| delete_doc(&client, event))).await
}
