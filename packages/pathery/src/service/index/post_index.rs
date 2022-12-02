use serde::Serialize;

use super::PathParams;
use crate::lambda::http::{self, err_response, HandlerResponse, HandlerResult, ServiceRequest};
use crate::schema::SchemaLoader;
use crate::search_doc::{SearchDoc, SearchDocError};
use crate::store::document::DocumentStore;
use crate::worker::index_writer::client::{IndexWriterClient, IndexWriterClientError};
use crate::worker::index_writer::job::Job;
use crate::{json, util};

#[derive(Serialize)]
pub struct PostIndexResponse {
    pub job_id: String,
    pub updated_at: String,
}

impl From<SearchDocError> for HandlerResponse {
    fn from(err: SearchDocError) -> Self {
        err_response(400, &err.to_string())
    }
}

impl From<IndexWriterClientError> for HandlerResponse {
    fn from(_: IndexWriterClientError) -> Self {
        todo!()
    }
}

// Indexes a document supplied via a JSON object in the body.
#[tracing::instrument(skip(writer_client, schema_loader, document_store))]
pub async fn post_index(
    document_store: &dyn DocumentStore,
    writer_client: &dyn IndexWriterClient,
    schema_loader: &dyn SchemaLoader,
    request: ServiceRequest<json::Value, PathParams>,
) -> HandlerResult {
    let (body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let schema = schema_loader.load_schema(&path_params.index_id);

    let document = match SearchDoc::from_json(&schema, body) {
        Ok(doc) => doc,
        Err(err) => return Ok(err.into()),
    };

    let doc_refs = match document_store.save_documents(vec![document]).await {
        Ok(refs) => refs,
        Err(err) => return Ok(err.into()),
    };

    let mut job = Job::create(&path_params.index_id);

    for doc_ref in doc_refs {
        job.index_doc(doc_ref);
    }

    let job_id = match writer_client.submit_job(job).await {
        Ok(job_id) => job_id,
        Err(err) => return Ok(err.into()),
    };

    http::success(&PostIndexResponse {
        job_id,
        updated_at: util::timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::test_utils::*;

    #[tokio::test]
    async fn post_index_doc_with_no_id() {
        let TestContext {
            document_store,
            index_writer_client,
            schema_loader,
            ..
        } = setup();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintenance",
            "author": "Robert Pirsig",
            "date_added": "2022-11-23T18:24:40Z",
            "isbn": "0060589469"
        });

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(
            document_store.as_ref(),
            &index_writer_client,
            schema_loader.as_ref(),
            request,
        )
        .await
        .unwrap();

        let (code, _body) = parse_response::<json::Value>(response);

        assert_eq!(200, code);
    }

    #[tokio::test]
    async fn post_index_non_object() {
        let TestContext {
            document_store,
            index_writer_client,
            schema_loader,
            ..
        } = setup();

        let doc = json::json!([]);

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(
            document_store.as_ref(),
            &index_writer_client,
            schema_loader.as_ref(),
            request,
        )
        .await
        .unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(400, code);
        assert_eq!(
            json::json!({"message": "json value is not an object"}),
            body
        );
    }

    #[tokio::test]
    async fn post_index_value_that_does_not_match_schema() {
        let TestContext {
            document_store,
            index_writer_client,
            schema_loader,
            ..
        } = setup();

        let doc = json::json!({"title": 1});

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(
            document_store.as_ref(),
            &index_writer_client,
            schema_loader.as_ref(),
            request,
        )
        .await
        .unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(400, code);
        assert_eq!(
            json::json!({"message": "The field '\"title\"' could not be parsed: TypeError { expected: \"a string\", json: Number(1) }"}),
            body,
        );
    }

    #[tokio::test]
    async fn post_index_field_that_does_not_exist() {
        let TestContext {
            document_store,
            index_writer_client,
            schema_loader,
            ..
        } = setup();

        let doc = json::json!({
            "foobar": "baz",
        });

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(
            document_store.as_ref(),
            &index_writer_client,
            schema_loader.as_ref(),
            request,
        )
        .await
        .unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(400, code);
        // Empty because the non-existent field does not explicitly trigger a failure - it just
        // doesn't get indexed.
        assert_eq!(
            json::json!({"message": "cannot index empty document"}),
            body,
        );
    }
}
