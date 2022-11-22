use std::env;
use std::marker::PhantomData;

use async_trait::async_trait;
use aws_lambda_events::bytes::Buf;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::json;

#[derive(Serialize, Deserialize, Debug)]
pub struct S3Ref {
    bucket: String,
    key: String,
}

#[async_trait]
pub trait S3Bucket<O>: Send + Sync {
    async fn write_object(&self, key: &str, obj: &O) -> Option<S3Ref>;

    async fn read_object(&self, key: &S3Ref) -> Option<O>;

    async fn delete_object(&self, key: &S3Ref);
}

pub struct S3BucketClient<O> {
    bucket: String,
    client: aws_sdk_s3::Client,
    object_type: PhantomData<O>,
}

#[async_trait]
impl<O> S3Bucket<O> for S3BucketClient<O>
where O: Serialize + for<'de> Deserialize<'de> + Sync + Send
{
    async fn write_object(&self, key: &str, obj: &O) -> Option<S3Ref> {
        let body = json::to_vec(obj).expect("object should serialize");

        let response = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await;

        match response {
            Ok(_) => Some(S3Ref {
                bucket: self.bucket.to_string(),
                key: key.to_string(),
            }),

            Err(err) => {
                error!(message = err.to_string());

                None
            }
        }
    }

    async fn read_object(&self, s3_ref: &S3Ref) -> Option<O> {
        let response = self
            .client
            .get_object()
            .bucket(&s3_ref.bucket)
            .key(&s3_ref.key)
            .send()
            .await;

        match response {
            Ok(response) => {
                let body = response
                    .body
                    .collect()
                    .await
                    .expect("body should be collected");

                let obj: O = json::from_reader(body.reader()).expect("object should deserialize");

                Some(obj)
            }
            Err(err) => {
                error!(message = err.to_string());

                return None;
            }
        }
    }

    async fn delete_object(&self, s3_ref: &S3Ref) {
        self.client
            .delete_object()
            .bucket(&s3_ref.bucket)
            .key(&s3_ref.key)
            .send()
            .await
            .expect("delete should succeed");
    }
}

pub async fn s3_bucket_client<O>() -> S3BucketClient<O> {
    let bucket_name = env::var("DATA_BUCKET_NAME").expect("should be set");
    let sdk_config = aws_config::load_from_env().await;

    S3BucketClient {
        bucket: bucket_name.into(),
        client: aws_sdk_s3::Client::new(&sdk_config),
        object_type: PhantomData,
    }
}

#[async_trait]
pub trait SQSQueue<O>: Send + Sync {
    async fn send_message(&self, group_id: &str, message: &O);
}

pub struct SQSQueueClient<O> {
    queue_url: String,
    client: aws_sdk_sqs::Client,
    object_type: PhantomData<O>,
}

#[async_trait]
impl<O> SQSQueue<O> for SQSQueueClient<O>
where O: Sync + Send + Serialize
{
    async fn send_message(&self, group_id: &str, message: &O) {
        let body = json::to_string(message).expect("message should serialize");

        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_group_id(group_id)
            .message_body(body)
            .send()
            .await
            .expect("message should queue");
    }
}

pub async fn sqs_queue_client<O>(queue_url: &str) -> SQSQueueClient<O> {
    let sdk_config = aws_config::load_from_env().await;

    SQSQueueClient {
        queue_url: queue_url.into(),
        client: aws_sdk_sqs::Client::new(&sdk_config),
        object_type: PhantomData,
    }
}
