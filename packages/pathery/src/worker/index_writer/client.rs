use std::env;

use async_trait::async_trait;
use serde_json as json;

use super::IndexWriterOp;

#[async_trait]
pub trait IndexWriterClient: Send + Sync {
    async fn send_message(&self, group_id: &str, message: IndexWriterOp);
}

pub struct AWSIndexWriterClient {
    queue_url: String,
    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl IndexWriterClient for AWSIndexWriterClient {
    async fn send_message(&self, group_id: &str, message: IndexWriterOp) {
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_group_id(group_id)
            .message_body(json::to_string(&message).expect("Message should serialize"))
            .send()
            .await
            .expect("send_message should not fail");
    }
}

pub async fn index_writer_client() -> AWSIndexWriterClient {
    let queue_url = env::var("QUEUE_URL").expect("QUEUE_URL should be set");
    let sdk_config = aws_config::load_from_env().await;
    let client = aws_sdk_sqs::Client::new(&sdk_config);
    AWSIndexWriterClient { queue_url, client }
}

#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
struct TestIndexWriterClient {
    ops: Arc<Mutex<Vec<IndexWriterOp>>>,
}

#[cfg(test)]
#[async_trait]
impl IndexWriterClient for TestIndexWriterClient {
    async fn send_message(&self, _group_id: &str, message: IndexWriterOp) {
        let mut ops = self.ops.lock().unwrap();
        (*ops).push(message);
    }
}

#[cfg(test)]
pub fn test_index_writer_client() -> impl IndexWriterClient {
    TestIndexWriterClient {
        ops: Arc::new(Mutex::new(Vec::new())),
    }
}
