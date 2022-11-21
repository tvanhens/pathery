use std::env;

use async_trait::async_trait;
use tantivy::Document;
use {serde, serde_json as json};

#[derive(serde::Deserialize, serde::Serialize)]
pub enum WriterMessageDetail {
    IndexSingleDoc { document: Document },
    DeleteSingleDoc { doc_id: String },
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct WriterMessage {
    pub index_id: String,
    pub detail: WriterMessageDetail,
}

impl WriterMessage {
    pub fn index_single_doc(index_id: &str, document: Document) -> Self {
        WriterMessage {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::IndexSingleDoc { document },
        }
    }

    pub fn delete_doc(index_id: &str, doc_id: &str) -> Self {
        WriterMessage {
            index_id: index_id.to_string(),
            detail: WriterMessageDetail::DeleteSingleDoc {
                doc_id: doc_id.to_string(),
            },
        }
    }
}

#[async_trait]
pub trait WriterSender: Send + Sync {
    async fn send_message(&self, group_id: &str, message: &WriterMessage);
}

pub struct LambdaWriterSender {
    queue_url: String,
    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl WriterSender for LambdaWriterSender {
    async fn send_message(&self, group_id: &str, message: &WriterMessage) {
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

pub async fn lambda_writer_sender() -> LambdaWriterSender {
    let queue_url = env::var("QUEUE_URL").expect("QUEUE_URL should be set");
    let sdk_config = aws_config::load_from_env().await;
    let client = aws_sdk_sqs::Client::new(&sdk_config);
    LambdaWriterSender { queue_url, client }
}

pub struct TestWriterSender {}

#[async_trait]
impl WriterSender for TestWriterSender {
    async fn send_message(&self, _group_id: &str, _message: &WriterMessage) {
        // NOOP
    }
}

pub fn test_writer_sender() -> TestWriterSender {
    TestWriterSender {}
}
