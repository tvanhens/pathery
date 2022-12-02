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
    use super::*;
    use crate::util;

    pub struct TestIndexWriterClient {}

    #[async_trait]
    impl IndexWriterClient for TestIndexWriterClient {
        async fn submit_job(&self, _job: Job) -> Result<String, IndexWriterClientError> {
            Ok(util::generate_id())
        }
    }

    impl TestIndexWriterClient {
        pub fn create() -> Self {
            TestIndexWriterClient {}
        }
    }
}
