use serde::Serialize;

use super::PathParams;
use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::schema::{SchemaExt, SchemaLoader};
use crate::worker::index_writer;
use crate::worker::index_writer::client::IndexWriterClient;
use crate::{json, util};

#[derive(Serialize)]
pub struct PostIndexResponse {
    #[serde(rename = "__id")]
    pub doc_id: String,
    pub updated_at: String,
}

// Indexes a document supplied via a JSON object in the body.
#[tracing::instrument(skip(writer_client, schema_loader))]
pub async fn post_index(
    writer_client: &IndexWriterClient,
    schema_loader: &dyn SchemaLoader,
    request: ServiceRequest<json::Value, PathParams>,
) -> HandlerResult {
    let (body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let schema = schema_loader.load_schema(&path_params.index_id);

    let (doc_id, index_doc) = match schema.to_document(body) {
        Ok(doc) => doc,
        Err(err) => return err.into(),
    };

    let mut batch = index_writer::batch(&path_params.index_id);

    batch.index_doc(index_doc);

    writer_client.write_batch(batch).await;

    http::success(&PostIndexResponse {
        doc_id,
        updated_at: util::timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::test_utils::*;

    #[tokio::test]
    async fn post_index_doc_with_no_id() {
        let (client, loader, _) = setup();

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

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, _body) = parse_response::<json::Value>(response);

        assert_eq!(code, 200);
    }

    #[tokio::test]
    async fn post_index_non_object() {
        let (client, loader, _) = setup();

        let doc = json::json!([]);

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        assert_eq!(body, json::json!({"message": "Expected JSON object"}));
    }

    #[tokio::test]
    async fn post_index_value_that_does_not_match_schema() {
        let (client, loader, _) = setup();

        let doc = json::json!({"title": 1});

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        assert_eq!(
            body,
            json::json!({"message": "The field '\"title\"' could not be parsed: TypeError { expected: \"a string\", json: Number(1) }"})
        );
    }

    #[tokio::test]
    async fn post_index_field_that_does_not_exist() {
        let (client, loader, _) = setup();

        let doc = json::json!({
            "foobar": "baz",
        });

        let request = request(
            doc,
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        // Empty because the non-existent field does not explicitly trigger a failure - it just
        // doesn't get indexed.
        assert_eq!(
            body,
            json::json!({"message": "Request JSON object is empty"})
        );
    }
}
