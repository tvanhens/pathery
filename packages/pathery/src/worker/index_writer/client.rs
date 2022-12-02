use async_trait::async_trait;
use thiserror::Error;

use super::job::Job;

#[derive(Debug, Error)]
pub enum IndexWriterClientError {}

#[async_trait]
pub trait IndexWriterClient: Sync + Send {
    async fn submit_job(&self, job: Job) -> Result<String, IndexWriterClientError>;
}

pub struct LambdaIndexWriterClient {}

#[async_trait]
impl IndexWriterClient for LambdaIndexWriterClient {
    async fn submit_job(&self, _job: Job) -> Result<String, IndexWriterClientError> {
        todo!()
    }
}

impl LambdaIndexWriterClient {
    pub fn create() -> LambdaIndexWriterClient {
        LambdaIndexWriterClient {}
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
