use pathery::service::index::QueryIndexService;
use pathery::service::start_service;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let service = QueryIndexService::create().await;

    start_service(&service).await
}
