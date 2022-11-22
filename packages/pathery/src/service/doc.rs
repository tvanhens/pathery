use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::util;
use crate::worker::index_writer;
use crate::worker::index_writer::client::IndexWriterClient;

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
    client: &IndexWriterClient,
    request: ServiceRequest<json::Value, PathParams>,
) -> HandlerResult {
    let (_body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let mut batch = index_writer::batch(&path_params.index_id);

    batch.delete_doc(&path_params.doc_id);

    client.write_batch(batch).await;

    http::success(&DeleteDocResponse {
        doc_id: path_params.doc_id,
        deleted_at: util::timestamp(),
    })
}
