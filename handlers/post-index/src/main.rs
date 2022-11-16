use pathery::anyhow::anyhow;
use pathery::chrono::{DateTime, Utc};
use pathery::index::{IndexProvider, TantivyIndex};
use pathery::json::Value;
use pathery::lambda::{self, http, http::PatheryRequest};
use pathery::tantivy::{Document, Term};
use pathery::{anyhow, json, serde, tokio, uuid};
use std::time::SystemTime;

fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

#[derive(serde::Serialize)]
#[serde(crate = "self::serde")]
struct PostIndexResponse {
    #[serde(rename = "__id")]
    doc_id: String,
    updated_at: String,
}

impl PostIndexResponse {
    fn new(doc_id: &str) -> PostIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        PostIndexResponse {
            doc_id: doc_id.to_string(),
            updated_at: now.to_rfc3339(),
        }
    }
}

pub fn index_doc(index_id: &str, raw_doc: &json::Value) -> anyhow::Result<String> {
    let index = IndexProvider::lambda_provider().load_index(index_id);
    let mut writer = index.default_writer();
    let schema = index.schema();

    let mut doc_obj = raw_doc.clone();
    let doc_obj = doc_obj
        .as_object_mut()
        .ok_or_else(|| anyhow!("Expected JSON object"))?;

    let id = doc_obj
        .remove("__id")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| generate_id());

    let id_field = index.id_field();

    let mut index_doc = Document::new();

    index_doc.add_text(id_field, &id);

    for (key, value) in doc_obj.iter() {
        if let Some(field) = schema.get_field(key) {
            match value {
                Value::String(v) => {
                    index_doc.add_text(field, v);
                    Ok(())
                }
                _ => Err(anyhow!("Unrecognized value: {:?}", value)),
            }?;
        }
    }

    writer.delete_term(Term::from_field_text(id_field, &id));
    writer.add_document(index_doc)?;

    writer.commit()?;

    Ok(id)
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::tracing_subscriber::fmt()
        .with_max_level(lambda::tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<json::Value>() {
            Ok(v) => v,
            Err(err) => return err.into(),
        };

        let doc_id = index_doc(&index_id, &payload)?;

        http::success(&PostIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
