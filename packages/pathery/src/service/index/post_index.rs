use async_trait::async_trait;
use serde::Serialize;

use crate::schema::{SchemaLoader, SchemaProvider};
use crate::search_doc::SearchDoc;
use crate::service::{ServiceError, ServiceHandler, ServiceRequest, ServiceResponse};
use crate::store::document::{DDBDocumentStore, DocumentStore};
use crate::worker::index_writer::client::{IndexWriterClient, LambdaIndexWriterClient};
use crate::worker::index_writer::job::Job;
use crate::{json, util};

#[derive(Serialize, Debug)]
pub struct PostIndexResponse {
    pub job_id: String,
    pub updated_at: String,
}

pub struct PostIndexService {
    schema_loader: Box<dyn SchemaLoader>,

    document_store: Box<dyn DocumentStore>,

    writer_client: Box<dyn IndexWriterClient>,
}

#[async_trait]
impl ServiceHandler<json::Value, PostIndexResponse> for PostIndexService {
    async fn handle_request(
        &self,
        request: ServiceRequest<json::Value>,
    ) -> ServiceResponse<PostIndexResponse> {
        let body = request.body()?;

        let index_id = request.path_param("index_id")?;

        let schema = self.schema_loader.load_schema(&index_id);

        let document = SearchDoc::from_json(&schema, body)
            .map_err(|err| ServiceError::invalid_request(&err.to_string()))?;

        let doc_refs = self.document_store.save_documents(vec![document]).await?;

        let mut job = Job::create(&index_id);

        for doc_ref in doc_refs {
            job.index_doc(doc_ref);
        }

        let job_id = self.writer_client.submit_job(job).await?;

        Ok(PostIndexResponse {
            job_id,
            updated_at: util::timestamp(),
        })
    }
}

impl PostIndexService {
    pub async fn create() -> Self {
        let document_store = DDBDocumentStore::create(None).await;
        let writer_client = LambdaIndexWriterClient::create(None).await;
        let schema_loader = SchemaProvider::lambda();

        PostIndexService {
            document_store: Box::new(document_store),
            writer_client: Box::new(writer_client),
            schema_loader: Box::new(schema_loader),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    pub fn test_service() -> PostIndexService {
        let ctx = setup();

        let schema_loader = Box::new(ctx.schema_loader().clone());
        let document_store = Box::new(ctx.document_store().clone());
        let writer_client = Box::new(ctx.writer_client().clone());

        PostIndexService {
            schema_loader,
            document_store,
            writer_client,
        }
    }

    #[tokio::test]
    async fn post_index_doc_with_no_id() {
        let service = test_service();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintenance",
            "author": "Robert Pirsig",
            "date_added": "2022-11-23T18:24:40Z",
            "isbn": "0060589469"
        });

        let request = ServiceRequest::create(doc).with_path_param("index_id", "test");

        service.handle_request(request).await.unwrap();
    }

    #[tokio::test]
    async fn post_index_non_object() {
        let service = test_service();

        let doc = json::json!([]);

        let request = ServiceRequest::create(doc).with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap_err();

        assert_eq!(
            ServiceError::invalid_request("json value is not an object"),
            response
        );
    }

    #[tokio::test]
    async fn post_index_value_that_does_not_match_schema() {
        let service = test_service();

        let doc = json::json!({"title": 1});

        let request = ServiceRequest::create(doc).with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap_err();

        assert_eq!(
            ServiceError::invalid_request(
                "The field '\"title\"' could not be parsed: TypeError { expected: \"a string\", \
                 json: Number(1) }"
            ),
            response
        );
    }

    #[tokio::test]
    async fn post_index_field_that_does_not_exist() {
        let service = test_service();

        let doc = json::json!({
            "foobar": "baz",
        });

        let request = ServiceRequest::create(doc).with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap_err();

        // Empty because the non-existent field does not explicitly trigger a failure - it just
        // doesn't get indexed.
        assert_eq!(
            ServiceError::invalid_request("cannot index empty document"),
            response,
        );
    }
}
