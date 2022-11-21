use crate::lambda::http::{self, HandlerResult, HttpRequest, PatheryRequest};
use crate::util;
use crate::worker::index_writer::client::IndexWriterClient;
use crate::worker::index_writer::op::IndexWriterOp;

#[derive(serde::Serialize)]
pub struct DeleteDocResponse {
    #[serde(rename = "__id")]
    pub doc_id: String,
    pub deleted_at: String,
}

pub async fn delete_doc(client: &dyn IndexWriterClient, request: HttpRequest) -> HandlerResult {
    let index_id = request.required_path_param("index_id");
    let doc_id = request.required_path_param("doc_id");

    client
        .send_message(IndexWriterOp::delete_doc(&index_id, &doc_id))
        .await;

    http::success(&DeleteDocResponse {
        doc_id,
        deleted_at: util::timestamp(),
    })
}
