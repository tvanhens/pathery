use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json as json;

use super::{ServiceHandler, ServiceRequest, ServiceResponse};
use crate::search_doc::SearchDocId;
use crate::worker::index_writer::client::{IndexWriterClient, LambdaIndexWriterClient};
use crate::worker::index_writer::job::Job;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    index_id: String,
    doc_id: String,
}

#[derive(Serialize)]
pub struct DeleteDocResponse {
    pub job_id: String,
}

pub struct DeleteDocService {
    client: Box<dyn IndexWriterClient>,
}

#[async_trait]
impl ServiceHandler<json::Value, DeleteDocResponse> for DeleteDocService {
    async fn handle_request(
        &self,
        request: ServiceRequest<json::Value>,
    ) -> ServiceResponse<DeleteDocResponse> {
        let index_id = request.path_param("index_id")?;
        let doc_id = request.path_param("doc_id")?;

        let mut job = Job::create(&index_id);

        job.delete_doc(SearchDocId::parse(&doc_id));

        let job_id = self.client.submit_job(job).await?;

        Ok(DeleteDocResponse { job_id })
    }
}

impl DeleteDocService {
    pub async fn create() -> Self {
        let client = LambdaIndexWriterClient::create(None).await;

        DeleteDocService {
            client: Box::new(client),
        }
    }
}
