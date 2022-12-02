use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::util;
use crate::worker::index_writer::client::IndexWriterClient;
use crate::worker::index_writer::job::Job;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    index_id: String,
    doc_id: String,
}

#[derive(Serialize)]
pub struct DeleteDocResponse {
    #[serde(rename = "__id")]
    pub doc_id: String,
    pub deleted_at: String,
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

    job.delete_doc(&path_params.doc_id);

    client.submit_job(job).await;

    http::success(&DeleteDocResponse {
        doc_id: path_params.doc_id,
        deleted_at: util::timestamp(),
    })
}
