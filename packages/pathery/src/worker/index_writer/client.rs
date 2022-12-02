use async_trait::async_trait;
use thiserror::Error;

use super::job::Job;
use crate::util;

#[derive(Debug, Error)]
pub enum IndexWriterClientError {}

#[async_trait]
pub trait IndexWriterClient: Sync + Send {
    async fn submit_job(&self, job: Job) -> Result<String, IndexWriterClientError>;
}

pub struct LambdaIndexWriterClient {
    queue_url: String,
    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl IndexWriterClient for LambdaIndexWriterClient {
    async fn submit_job(&self, job: Job) -> Result<String, IndexWriterClientError> {
        let body = serde_json::to_string(&job).expect("job should serialize");

        let response = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .message_group_id(job.index_id)
            .send()
            .await
            .expect("job should queue");

        Ok(response
            .message_id()
            .expect("message id should exist")
            .to_string())
    }
}

impl LambdaIndexWriterClient {
    pub async fn create(queue_url: Option<&str>) -> LambdaIndexWriterClient {
        let sdk_config = aws_config::load_from_env().await;

        LambdaIndexWriterClient {
            queue_url: queue_url
                .map(String::from)
                .unwrap_or_else(|| util::require_env("INDEX_WRITER_QUEUE_URL")),
            client: aws_sdk_sqs::Client::new(&sdk_config),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::sync::Arc;

    use tantivy::Index;

    use super::*;
    use crate::index::IndexExt;
    use crate::store::document::test_util::TestDocumentStore;
    use crate::util;
    use crate::worker::index_writer::handle_job;

    pub struct TestIndexWriterClient {
        index: Arc<Index>,
        document_store: Arc<TestDocumentStore>,
    }

    #[async_trait]
    impl IndexWriterClient for TestIndexWriterClient {
        async fn submit_job(&self, job: Job) -> Result<String, IndexWriterClientError> {
            let mut writer = self.index.default_writer();

            handle_job(&mut writer, self.document_store.as_ref(), job).await;

            Ok(util::generate_id())
        }
    }

    impl TestIndexWriterClient {
        pub fn create(index: &Arc<Index>, document_store: &Arc<TestDocumentStore>) -> Self {
            TestIndexWriterClient {
                index: Arc::clone(index),
                document_store: Arc::clone(document_store),
            }
        }
    }
}
