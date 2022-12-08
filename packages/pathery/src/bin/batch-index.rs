use pathery::service::index::BatchIndexService;
use pathery::service::start_service;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let service = BatchIndexService::create().await;

    start_service(&service).await
}
