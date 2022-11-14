use std::sync::Arc;

use aws_sdk_dynamodb::Client as DDBClient;

pub async fn ddb_client() -> Arc<DDBClient> {
    let config = aws_config::load_from_env().await;
    Arc::new(aws_sdk_dynamodb::Client::new(&config))
}
