use crate::message::WriterMessage;
use async_trait::async_trait;
use serde_json as json;
use std::env;

#[async_trait]
pub trait SQSQueueSender {
    async fn send_fifo(&self, group_id: &str, message: &WriterMessage);
}

pub struct QueueClient {
    queue_url: String,
    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl SQSQueueSender for QueueClient {
    async fn send_fifo(&self, group_id: &str, message: &WriterMessage) {
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_group_id(group_id)
            .message_body(json::to_string(message).expect("Message should serialize"))
            .send()
            .await
            .expect("send_message should not fail");
    }
}

pub async fn lambda_queue_client() -> QueueClient {
    let queue_url = env::var("QUEUE_URL").expect("QUEUE_URL should be set");
    let sdk_config = aws_config::load_from_env().await;
    let client = aws_sdk_sqs::Client::new(&sdk_config);
    QueueClient { queue_url, client }
}

pub struct TestQueueClient {}

#[async_trait]
impl SQSQueueSender for TestQueueClient {
    async fn send_fifo(&self, _group_id: &str, _message: &WriterMessage) {
        // NOOP
    }
}

pub fn test_queue_client() -> TestQueueClient {
    TestQueueClient {}
}
