use std::env;

use super::op::OpBatch;
use crate::aws::{s3_bucket_client, sqs_queue_client, S3Bucket, S3Ref, SQSQueue};
use crate::util;

pub struct IndexWriterClient {
    bucket_client: Box<dyn S3Bucket<OpBatch>>,
    queue_client: Box<dyn SQSQueue<S3Ref>>,
}

impl IndexWriterClient {
    pub async fn default() -> IndexWriterClient {
        let queue_url = env::var("INDEX_WRITER_QUEUE_URL").expect("should be set");

        IndexWriterClient {
            bucket_client: Box::new(s3_bucket_client().await),
            queue_client: Box::new(sqs_queue_client(&queue_url).await),
        }
    }

    pub async fn write_batch(&self, batch: OpBatch) {
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
