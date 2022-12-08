use async_trait::async_trait;
use serde::Serialize;

use crate::json;
use crate::schema::{SchemaLoader, SchemaProvider};
use crate::search_doc::SearchDoc;
use crate::service::{ServiceError, ServiceHandler, ServiceRequest, ServiceResponse};
use crate::store::document::{DDBDocumentStore, DocumentStore};
use crate::worker::index_writer::client::{IndexWriterClient, LambdaIndexWriterClient};
use crate::worker::index_writer::job::Job;

#[derive(Serialize)]
pub struct BatchIndexResponse {
    pub job_id: String,
}

pub struct BatchIndexService {
    schema_loader: Box<dyn SchemaLoader>,

    document_store: Box<dyn DocumentStore>,

    index_writer: Box<dyn IndexWriterClient>,
}

#[async_trait]
impl ServiceHandler<Vec<json::Value>, BatchIndexResponse> for BatchIndexService {
    async fn handle_request(
        &self,
        request: ServiceRequest<Vec<json::Value>>,
    ) -> ServiceResponse<BatchIndexResponse> {
        let body = request.body()?;

        let index_id = request.path_param("index_id")?;

        let schema = self.schema_loader.load_schema(&index_id);

        let mut job = Job::create(&index_id);

        let documents = body
            .into_iter()
            .map(|value| SearchDoc::from_json(&schema, value))
            .collect::<Vec<_>>();

        let error = documents
            .iter()
            .enumerate()
            .filter_map(|(idx, result)| result.as_ref().err().map(|err| (idx, err)))
            .collect::<Vec<_>>();

        if let Some((idx, error)) = error.first() {
            return Err(ServiceError::invalid_request(&format!(
                "Error parsing document (path: [{}]): {}",
                idx,
                error.to_string()
            )));
        }

        let documents = documents
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let doc_refs = self.document_store.save_documents(documents).await?;

        for doc_ref in doc_refs {
            job.index_doc(doc_ref)
        }

        let job_id = self.index_writer.submit_job(job).await?;

        Ok(BatchIndexResponse { job_id })
    }
}

impl BatchIndexService {
    pub async fn create() -> Self {
        let document_store = DDBDocumentStore::create(None).await;
        let writer_client = LambdaIndexWriterClient::create(None).await;
        let schema_loader = SchemaProvider::lambda();

        BatchIndexService {
            document_store: Box::new(document_store),
            index_writer: Box::new(writer_client),
            schema_loader: Box::new(schema_loader),
        }
    }
}
