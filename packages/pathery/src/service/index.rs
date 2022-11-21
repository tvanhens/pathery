use crate::lambda::http::{self, PatheryRequest};
use crate::message::{WriterMessage, WriterSender};
use crate::schema::{SchemaLoader, TantivySchema};
use chrono::{DateTime, Utc};
use serde;
use serde_json as json;
use serde_json::Value;
use std::fmt::Debug;
use std::time::SystemTime;
use tantivy::Document;
use tracing;

fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

#[derive(Debug, PartialEq, Eq)]
pub enum IndexError {
    EmptyObject,
    NotJsonObject,
    UnsupportedJsonValue,
}

impl From<IndexError> for Result<http::Response<http::Body>, http::Error> {
    fn from(err: IndexError) -> Self {
        match err {
            IndexError::EmptyObject => Ok(http::err_response(400, "Cannot index empty object")),
            IndexError::NotJsonObject => Ok(http::err_response(400, "Expected a JSON object")),
            IndexError::UnsupportedJsonValue => {
                Ok(http::err_response(400, "Unsupported JSON value in object"))
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(crate = "self::serde")]
pub struct PostIndexResponse {
    #[serde(rename = "__id")]
    doc_id: String,
    updated_at: String,
}

impl PostIndexResponse {
    pub fn new(doc_id: &str) -> PostIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        PostIndexResponse {
            doc_id: doc_id.to_string(),
            updated_at: now.to_rfc3339(),
        }
    }
}

#[tracing::instrument(skip(writer_client, schema_loader))]
pub async fn post_index(
    writer_client: &dyn WriterSender,
    schema_loader: &dyn SchemaLoader,
    request: http::Request,
) -> Result<http::Response<http::Body>, http::Error> {
    let index_id = request.required_path_param("index_id");

    let payload = match request.payload::<json::Value>() {
        Ok(v) => v,
        Err(err) => return err.into(),
    };

    let doc_id = match index_doc(writer_client, schema_loader, &index_id, &payload).await {
        Ok(v) => v,
        Err(err) => return err.into(),
    };

    http::success(&PostIndexResponse::new(&doc_id))
}

async fn index_doc(
    client: &dyn WriterSender,
    schema_loader: &dyn SchemaLoader,
    index_id: &str,
    raw_doc: &json::Value,
) -> Result<String, IndexError> {
    let schema = schema_loader.load_schema(index_id);

    let mut doc_obj = raw_doc.clone();
    let doc_obj = doc_obj.as_object_mut().ok_or(IndexError::NotJsonObject)?;

    let id = doc_obj
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
            _ => return Err(IndexError::UnsupportedJsonValue),
        };
    }

    if index_doc.is_empty() {
        // There are no fields that match the schema so the doc is empty
        return Err(IndexError::EmptyObject);
    }

    index_doc.add_text(id_field, &id);

    client
        .send_message(
            index_id,
            &WriterMessage::index_single_doc(index_id, index_doc),
        )
        .await;

    Ok(id.to_string())
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
