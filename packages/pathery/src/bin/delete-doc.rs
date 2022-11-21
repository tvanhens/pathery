use pathery::lambda;
use pathery::message::lambda_writer_sender;
use pathery::service::doc::delete_doc;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let client = lambda_writer_sender().await;

    lambda_http::run(lambda_http::service_fn(|event| delete_doc(&client, event))).await
}
