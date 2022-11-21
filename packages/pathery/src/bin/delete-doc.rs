use pathery::lambda;
use pathery::service::doc::delete_doc;
use pathery::worker::index_writer::client::index_writer_client;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let client = index_writer_client().await;

    lambda_http::run(lambda_http::service_fn(|event| delete_doc(&client, event))).await
}