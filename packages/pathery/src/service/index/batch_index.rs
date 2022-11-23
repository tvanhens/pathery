use serde::Serialize;

use super::PathParams;
use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::schema::{SchemaExt, SchemaLoader};
use crate::worker::index_writer;
use crate::worker::index_writer::client::IndexWriterClient;
use crate::{json, util};

#[derive(Serialize)]
pub struct BatchIndexResponse {
    #[serde(rename = "__id")]
    pub updated_at: String,
}

// Indexes a batch of documents
#[tracing::instrument(skip(writer_client, schema_loader))]
pub async fn batch_index(
    writer_client: &IndexWriterClient,
    schema_loader: &dyn SchemaLoader,
    request: ServiceRequest<Vec<json::Value>, PathParams>,
) -> HandlerResult {
    let (body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let schema = schema_loader.load_schema(&path_params.index_id);

    let mut batch = index_writer::batch(&path_params.index_id);

    for doc_obj in body.into_iter() {
        let (_id, document) = match schema.to_document(doc_obj) {
            Ok(doc) => doc,
            Err(err) => return err.into(),
        };

        batch.index_doc(document);
    }

    writer_client.write_batch(batch).await;

    http::success(&BatchIndexResponse {
        updated_at: util::timestamp(),
    })
}
