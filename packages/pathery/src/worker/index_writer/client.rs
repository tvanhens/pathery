use std::env;
use std::error::Error;

use async_trait::async_trait;

use super::job::Job;
use crate::aws::{s3_bucket_client, sqs_queue_client, S3Bucket, S3Ref, SQSQueue};
use crate::util;

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

pub struct DefaultClient {
    pub(crate) bucket_client: Box<dyn S3Bucket<Job>>,
    pub(crate) queue_client: Box<dyn SQSQueue<S3Ref>>,
}

impl DefaultClient {
    pub async fn default() -> DefaultClient {
        let queue_url = env::var("INDEX_WRITER_QUEUE_URL").expect("should be set");

        DefaultClient {
            bucket_client: Box::new(s3_bucket_client().await),
            queue_client: Box::new(sqs_queue_client(&queue_url).await),
        }
    }

    pub async fn write_batch(&self, batch: Job) {
        let key = format!("writer_batches/{}", util::generate_id());

        let s3_ref = self
            .bucket_client
            .write_object(&key, &batch)
            .await
            .expect("object should write");

        self.queue_client
            .send_message(&batch.index_id, &s3_ref)
            .await;
    }
}
