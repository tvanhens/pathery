use pathery::chrono::{DateTime, Utc};
use pathery::json::Value;
use pathery::lambda::http;
use pathery::lambda::tracing;
use pathery::message::{WriterMessage, WriterSender};
use pathery::schema::{SchemaLoader, TantivySchema};
use pathery::tantivy::Document;
use pathery::{json, serde, uuid};
use std::fmt::Debug;
use std::time::SystemTime;

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

#[tracing::instrument(skip(client, schema_loader, raw_doc))]
pub async fn index_doc<C, L>(
    client: &C,
    schema_loader: &L,
    index_id: &str,
    raw_doc: &json::Value,
) -> Result<String, IndexError>
where
    C: WriterSender,
    L: SchemaLoader,
{
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
    use pathery::{
        message::{test_writer_sender, TestWriterSender},
        schema::{test_schema_loader, TestSchemaLoader},
        tokio,
    };
    fn setup() -> (TestWriterSender, TestSchemaLoader) {
        (test_writer_sender(), test_schema_loader())
    }

    // Happy Path

    #[tokio::test]
    async fn index_a_doc_with_no_id() {
        let (client, loader) = setup();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintenance",
            "author": "Robert Pirsig"
        });

        let result = index_doc(&client, &loader, "test", &doc).await;

        result.expect("result should not be an error");
    }

    // Error States

    #[tokio::test]
    async fn index_a_non_object() {
        let (client, loader) = setup();

        let doc = json::json!([]);

        let result = index_doc(&client, &loader, "test", &doc).await;

        assert_eq!(result, Err(IndexError::NotJsonObject));
    }

    #[tokio::test]
    async fn index_an_unsupported_value() {
        let (client, loader) = setup();

        let doc = json::json!({"foo": 1});

        let result = index_doc(&client, &loader, "test", &doc).await;

        assert_eq!(result, Err(IndexError::UnsupportedJsonValue));
    }

    #[tokio::test]
    async fn index_a_field_that_does_not_exist() {
        let (client, loader) = setup();

        let doc = json::json!({
            "foobar": "baz",
        });

        let result = index_doc(&client, &loader, "test", &doc).await;

        assert_eq!(result, Err(IndexError::EmptyObject));
    }
}
