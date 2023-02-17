use std::fmt::Debug;

use async_trait::async_trait;

use super::job::AsyncDeleteJob;
use crate::service::ServiceError;
use crate::util;

#[async_trait]
pub trait AsyncDeleteClient: Sync + Send + Debug {
    async fn submit_job(&self, job: AsyncDeleteJob) -> Result<String, ServiceError>;
}

#[derive(Debug)]
pub struct LambdaAsyncDeleteClient {
    queue_url: String,

    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl AsyncDeleteClient for LambdaAsyncDeleteClient {
    async fn submit_job(&self, job: AsyncDeleteJob) -> Result<String, ServiceError> {
        let body = serde_json::to_string(&job).expect("job should serialize");

        let response = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .send()
            .await
            .expect("job should queue");

        Ok(response
            .message_id()
            .expect("message id should exist")
            .to_string())
    }
}

impl LambdaAsyncDeleteClient {
    pub async fn create(queue_url: Option<&str>) -> LambdaAsyncDeleteClient {
        let sdk_config = aws_config::load_from_env().await;

        LambdaAsyncDeleteClient {
            queue_url: queue_url
                .map(String::from)
                .unwrap_or_else(|| util::require_env("ASYNC_DELETE_QUEUE_URL")),
            client: aws_sdk_sqs::Client::new(&sdk_config),
        }
    }
}
