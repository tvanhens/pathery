use std::error::Error;

use async_trait::async_trait;

use super::job::Job;

#[async_trait]
pub trait IndexWriterClient: Sync + Send {
    async fn submit_job(&self, job: Job) -> Result<String, Box<dyn Error>>;
}

pub struct LambdaIndexWriterClient {}

#[async_trait]
impl IndexWriterClient for LambdaIndexWriterClient {
    async fn submit_job(&self, _job: Job) -> Result<String, Box<dyn Error>> {
        todo!()
    }
}

impl LambdaIndexWriterClient {
    pub fn create() -> LambdaIndexWriterClient {
        LambdaIndexWriterClient {}
    }
}
