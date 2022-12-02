use serde::Serialize;

use super::PathParams;
use crate::json;
use crate::lambda::http::{self, err_response, HandlerResponse, HandlerResult, ServiceRequest};
use crate::schema::SchemaLoader;
use crate::search_doc::SearchDoc;
use crate::store::document::{DocumentStore, DocumentStoreError};
use crate::worker::index_writer::client::IndexWriterClient;
use crate::worker::index_writer::job::Job;

#[derive(Serialize)]
pub struct BatchIndexResponse {
    pub job_id: String,
}

impl From<DocumentStoreError> for HandlerResponse {
    fn from(err: DocumentStoreError) -> Self {
        err_response(500, &format!("Error storing document: {}", err.to_string()))
    }
}

// Indexes a batch of documents
#[tracing::instrument(skip(index_writer, schema_loader, document_store))]
pub async fn batch_index(
    document_store: &dyn DocumentStore,
    index_writer: &dyn IndexWriterClient,
    schema_loader: &dyn SchemaLoader,
    request: ServiceRequest<Vec<json::Value>, PathParams>,
) -> HandlerResult {
    let (body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let schema = schema_loader.load_schema(&path_params.index_id);

    let mut job = Job::create(&path_params.index_id);

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
        return Ok(err_response(
            400,
            &format!(
                "Error parsing document (path: [{}]): {}",
                idx,
                error.to_string()
            ),
        ));
    }

    let documents = documents
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let doc_refs = match document_store.save_documents(documents).await {
        Ok(refs) => refs,
        Err(err) => return Ok(err.into()),
    };

    job.index_batch(doc_refs);

    match index_writer.submit_job(job).await {
        Ok(job_id) => http::success(&BatchIndexResponse { job_id }),
        _ => Ok(err_response(500, "Error submitting job")),
    }
}
