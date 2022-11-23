pub mod doc;
pub mod index;

#[cfg(test)]
mod test_utils {
    use std::collections::HashMap;
    use std::marker::PhantomData;
    use std::vec;

    use ::http::{Request, StatusCode};
    use async_trait::async_trait;
    use aws_lambda_events::query_map::QueryMap;
    use lambda_http::{Body, RequestExt};
    use serde::{Deserialize, Serialize};
    pub use tantivy::doc;

    use crate::aws::{S3Bucket, S3Ref, SQSQueue};
    pub(crate) use crate::json;
    use crate::lambda::http::{HandlerResponse, HttpRequest, ServiceRequest};
    use crate::schema::SchemaProvider;
    use crate::worker::index_writer::client::IndexWriterClient;

    fn test_index_writer_client() -> IndexWriterClient {
        struct TestBucketClient<O> {
            object_type: PhantomData<O>,
        }

        #[async_trait]
        impl<O: Send + Sync> S3Bucket<O> for TestBucketClient<O> {
            async fn write_object(&self, key: &str, _obj: &O) -> Option<S3Ref> {
                Some(S3Ref {
                    bucket: "test".into(),
                    key: key.into(),
                })
            }

            async fn read_object(&self, _s3_ref: &S3Ref) -> Option<O> {
                todo!()
            }

            async fn delete_object(&self, _s3_ref: &S3Ref) {
                todo!()
            }
        }

        struct TestQueueClient<O> {
            object_type: PhantomData<O>,
        }

        #[async_trait]
        impl<O: Send + Sync> SQSQueue<O> for TestQueueClient<O> {
            async fn send_message(&self, _group_id: &str, _message: &O) {}
        }

        IndexWriterClient {
            bucket_client: Box::new(TestBucketClient {
                object_type: PhantomData,
            }),
            queue_client: Box::new(TestQueueClient {
                object_type: PhantomData,
            }),
        }
    }

    pub fn setup() -> (IndexWriterClient, SchemaProvider) {
        let config = json::json!({
            "indexes": [
                {
                    "prefix": "test",
                    "fields": [
                        {
                            "name": "title",
                            "kind": "text",
                            "flags": ["TEXT"]
                        },
                        {
                            "name": "author",
                            "kind": "text",
                            "flags": ["TEXT"]
                        }
                    ]
                }
            ]
        });
        (
            test_index_writer_client(),
            SchemaProvider::from_json(config),
        )
    }

    pub fn request<B, P>(body: B, params: P) -> ServiceRequest<B, P>
    where
        B: Serialize,
        P: Serialize,
    {
        let request: HttpRequest = Request::builder()
            .header("Content-Type", "application/json")
            .body(json::to_string(&body).expect("should serialize").into())
            .expect("should build request");

        let params_value = json::to_value(params).expect("params should serialize to value");

        let params_map: HashMap<String, String> =
            json::from_value(params_value).expect("params value should deserialize");

        request
            .with_path_parameters::<QueryMap>(params_map.into())
            .into()
    }

    pub fn parse_response<V>(response: HandlerResponse) -> (StatusCode, V)
    where V: for<'de> Deserialize<'de> {
        let code = response.status();
        let body: V = if let Body::Text(x) = response.body() {
            json::from_str(x).unwrap()
        } else {
            panic!("Invalid body")
        };
        (code, body)
    }
}
