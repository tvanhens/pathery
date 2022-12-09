use pathery::service::doc::DeleteDocService;
use pathery::service::start_service;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let service = DeleteDocService::create().await;

    start_service(&service).await
}
