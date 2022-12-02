use pathery::lambda;
use pathery::lambda::http::HttpRequest;
use pathery::service::doc::delete_doc;
use pathery::worker::index_writer::client::LambdaIndexWriterClient;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    lambda::init_tracing();

    let client = LambdaIndexWriterClient::create(None).await;

    lambda_http::run(lambda_http::service_fn(|event: HttpRequest| {
        delete_doc(&client, event.into())
    }))
    .await
}
