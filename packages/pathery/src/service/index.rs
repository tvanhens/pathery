use crate::lambda::http::{self, PatheryRequest};
use crate::message::{WriterMessage, WriterSender};
use crate::schema::{SchemaLoader, TantivySchema};
use chrono::{DateTime, Utc};
use serde;
use serde_json as json;
use serde_json::Value;
use std::time::SystemTime;
use tantivy::Document;
use tracing;

trait IndexResourceRequest {
    fn index_id(&self) -> String;
}

impl IndexResourceRequest for http::Request {
    fn index_id(&self) -> String {
        self.required_path_param("index_id")
    }
}

fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

fn timestamp() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}

#[derive(serde::Serialize)]
pub struct PostIndexResponse {
    #[serde(rename = "__id")]
    pub doc_id: String,
    pub updated_at: String,
}

// Indexes a document supplied via a JSON object in the body.
#[tracing::instrument(skip(writer_client, schema_loader))]
pub async fn post_index(
    writer_client: &dyn WriterSender,
    schema_loader: &dyn SchemaLoader,
    request: http::Request,
) -> Result<http::Response<http::Body>, http::Error> {
    let index_id = request.index_id();

    let mut payload = match request.payload::<json::Value>() {
        Ok(v) => v,
        Err(err) => return err.into(),
    };

    let schema = schema_loader.load_schema(&index_id);

    let doc_obj = if let Some(obj) = payload.as_object_mut() {
        obj
    } else {
        return Ok(http::err_response(400, "Expected a JSON object"));
    };

    let doc_id = doc_obj
        .remove("__id")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(generate_id);

    let id_field = schema.id_field();

    let mut index_doc = Document::new();

    for (key, value) in doc_obj.iter() {
        match value {
            Value::String(v) => {
                if let Some(field) = schema.get_field(key) {
                    index_doc.add_text(field, v);
                }
            }
            _ => return Ok(http::err_response(400, "Unsupported JSON value in object")),
        };
    }

    if index_doc.is_empty() {
        // There are no fields that match the schema so the doc is empty
        return Ok(http::err_response(400, "Cannot index empty object"));
    }

    index_doc.add_text(id_field, &doc_id);

    writer_client
        .send_message(
            &index_id,
            &WriterMessage::index_single_doc(&index_id, index_doc),
        )
        .await;

    http::success(&PostIndexResponse {
        doc_id,
        updated_at: timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message::{test_writer_sender, TestWriterSender},
        schema::SchemaProvider,
        tokio,
    };
    use ::http::{Request, StatusCode};
    use aws_lambda_events::query_map::QueryMap;
    use lambda_http::{Body, RequestExt};
    use std::collections::HashMap;

    fn setup() -> (TestWriterSender, SchemaProvider) {
        let config = json::json!({
            "indexes": [
                {
                    "prefix": "test",
                    "fields": [
                        {
                            "name": "title",
                            "kind": "text",
                            "flags": ["TEXT"]
                        },
                        {
                            "name": "author",
                            "kind": "text",
                            "flags": ["TEXT"]
                        }
                    ]
                }
            ]
        });
        (test_writer_sender(), SchemaProvider::from_json(config))
    }

    fn request(index_id: &str, body: json::Value) -> http::Request {
        let request: http::Request = Request::builder()
            .header("Content-Type", "application/json")
            .body(json::to_string(&body).expect("should serialize").into())
            .expect("should build request");

        request.with_path_parameters::<QueryMap>(
            HashMap::from([(String::from("index_id"), String::from(index_id))]).into(),
        )
    }

    fn parse_response(response: http::Response<http::Body>) -> (StatusCode, json::Value) {
        let code = response.status();
        let body: json::Value = if let Body::Text(x) = response.body() {
            json::from_str(x).unwrap()
        } else {
            panic!("Invalid body")
        };
        (code, body)
    }

    // Happy Path

    #[tokio::test]
    async fn index_a_doc_with_no_id() {
        let (client, loader) = setup();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintenance",
            "author": "Robert Pirsig"
        });

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, _body) = parse_response(response);

        assert_eq!(code, 200);
    }

    // Error States

    #[tokio::test]
    async fn index_a_non_object() {
        let (client, loader) = setup();

        let doc = json::json!([]);

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response(response);

        assert_eq!(code, 400);
        assert_eq!(body, json::json!({"message": "Expected a JSON object"}));
    }

    #[tokio::test]
    async fn index_an_unsupported_value() {
        let (client, loader) = setup();

        let doc = json::json!({"foo": 1});

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response(response);

        assert_eq!(code, 400);
        assert_eq!(
            body,
            json::json!({"message": "Unsupported JSON value in object"})
        );
    }

    #[tokio::test]
    async fn index_a_field_that_does_not_exist() {
        let (client, loader) = setup();

        let doc = json::json!({
            "foobar": "baz",
        });

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response(response);

        assert_eq!(code, 400);
        // Empty because the non-existent field does not explicitly trigger a failure - it just doesn't get indexed.
        assert_eq!(body, json::json!({"message": "Cannot index empty object"}));
    }
}
