use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::search_doc::SearchDocId;
use crate::worker::index_writer::client::IndexWriterClient;
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

pub async fn delete_doc(
    client: &dyn IndexWriterClient,
    request: ServiceRequest<json::Value, PathParams>,
) -> HandlerResult {
    let (_body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let mut job = Job::create(&path_params.index_id);

    job.delete_doc(SearchDocId::parse(&path_params.doc_id));

    let job_id = match client.submit_job(job).await {
        Ok(job_id) => job_id,
        Err(_) => todo!(),
    };

    http::success(&DeleteDocResponse { job_id })
}
