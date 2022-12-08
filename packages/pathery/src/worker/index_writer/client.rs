use async_trait::async_trait;
use thiserror::Error;

use super::job::Job;
use crate::service::ServiceError;
use crate::util;

#[derive(Debug, Error)]
pub enum IndexWriterClientError {}

#[async_trait]
pub trait IndexWriterClient: Sync + Send {
    async fn submit_job(&self, job: Job) -> Result<String, ServiceError>;
}

pub struct LambdaIndexWriterClient {
    queue_url: String,
    client: aws_sdk_sqs::Client,
}

#[async_trait]
impl IndexWriterClient for LambdaIndexWriterClient {
    async fn submit_job(&self, job: Job) -> Result<String, ServiceError> {
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
    use super::*;
    use crate::index::test_util::TestIndexLoader;
    use crate::index::{IndexExt, IndexLoader};
    use crate::store::document::test_util::TestDocumentStore;
    use crate::util;
    use crate::worker::index_writer::handle_job;

    #[derive(Clone)]
    pub struct TestIndexWriterClient {
        index_loader: TestIndexLoader,

        document_store: TestDocumentStore,
    }

    #[async_trait]
    impl IndexWriterClient for TestIndexWriterClient {
        async fn submit_job(&self, job: Job) -> Result<String, ServiceError> {
            let index = self.index_loader.load_index(&job.index_id, None);

            let mut writer = index.default_writer();

            handle_job(&mut writer, &self.document_store, job).await;

            writer.commit().unwrap();

            Ok(util::generate_id())
        }
    }

    impl TestIndexWriterClient {
        pub fn create(index_loader: TestIndexLoader, document_store: TestDocumentStore) -> Self {
            TestIndexWriterClient {
                index_loader,
                document_store,
            }
        }
    }
}
