use pathery::chrono::{DateTime, Utc};
use pathery::index::TantivyIndex;
use pathery::json::Value;
use pathery::lambda::http;
use pathery::tantivy::{Document, Index, Term};
use pathery::{json, serde, uuid};
use std::time::SystemTime;

fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

#[derive(Debug, PartialEq)]
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

pub fn index_doc(index: &Index, raw_doc: &json::Value) -> Result<String, IndexError> {
    let mut writer = index.default_writer();
    let schema = index.schema();

    let mut doc_obj = raw_doc.clone();
    let doc_obj = doc_obj
        .as_object_mut()
        .ok_or_else(|| IndexError::NotJsonObject)?;

    let id = doc_obj
        .remove("__id")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| generate_id());

    let id_field = index.id_field();

    let mut index_doc = Document::new();

    for (key, value) in doc_obj.iter() {
        if let Some(field) = schema.get_field(key) {
            match value {
                Value::String(v) => {
                    index_doc.add_text(field, v);
                    Ok(())
                }
                _ => Err(IndexError::UnsupportedJsonValue),
            }?;
        }
    }

    if index_doc.len() < 1 {
        // There are no fields that match the schema so the doc is empty
        return Err(IndexError::EmptyObject);
    }

    index_doc.add_text(id_field, &id);

    writer.delete_term(Term::from_field_text(id_field, &id));
    writer
        .add_document(index_doc)
        .expect("Adding a document should not error");

    writer.commit().expect("Commit should not error");

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pathery::index::test_index;

    #[test]
    fn index_a_doc_with_no_id() {
        let index = test_index();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintentance",
            "author": "Robert Pirsig"
        });

        let result = index_doc(&index, &doc);

        result.expect("result should not be an error");

        assert_eq!(1, index.reader().unwrap().searcher().num_docs());
    }

    #[test]
    fn index_a_field_that_does_not_exist() {
        let index = test_index();

        let doc = json::json!({
            "foobar": "Zen and the Art of Motorcycle Maintentance",
        });

        let result = index_doc(&index, &doc);

        assert_eq!(result, Err(IndexError::EmptyObject));

        assert_eq!(0, index.reader().unwrap().searcher().num_docs());
    }
}
