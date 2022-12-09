use pathery::service::index::PostIndexService;
use pathery::service::start_service;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let service = PostIndexService::create().await;

    start_service(&service).await
}
