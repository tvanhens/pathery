use pathery::chrono::{DateTime, Utc};
use pathery::index::{IndexProvider, TantivyIndex};
use pathery::lambda::{http, http::PatheryRequest, tracing, tracing_subscriber};
use pathery::tantivy::Term;
use pathery::{anyhow, serde, tokio};
use std::time::SystemTime;

#[derive(serde::Serialize)]
#[serde(crate = "self::serde")]
struct DeleteIndexResponse {
    #[serde(rename = "__id")]
    doc_id: String,
    deleted_at: String,
}

impl DeleteIndexResponse {
    fn new(doc_id: &str) -> DeleteIndexResponse {
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        DeleteIndexResponse {
            doc_id: doc_id.to_string(),
            deleted_at: now.to_rfc3339(),
        }
    }
}

fn delete_doc(index_id: &str, doc_id: &str) -> anyhow::Result<()> {
    let index = IndexProvider::lambda_provider().load_index(index_id);
    let mut writer = index.default_writer();
    let id_field = index.id_field();
    writer.delete_term(Term::from_field_text(id_field, doc_id));
    writer.commit()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");
        let doc_id = event.required_path_param("doc_id");

        delete_doc(&index_id, &doc_id)?;

        http::success(&DeleteIndexResponse::new(&doc_id))
    };

    http::run(http::service_fn(handler)).await
}
