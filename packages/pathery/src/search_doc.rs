use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tantivy::schema::{DocParsingError, Schema};
use tantivy::Document;
use thiserror::Error;

use crate::util;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SearchDocError {
    #[error("json value is not an object")]
    NotAnObject,

    #[error("invalid type for __id, expected string")]
    InvalidIdType,

    #[error("{0}")]
    SchemaValidationError(String),

    #[error("cannot index empty document")]
    EmptyDocument,
}

impl From<DocParsingError> for SearchDocError {
    fn from(err: DocParsingError) -> Self {
        SearchDocError::SchemaValidationError(err.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DDBKey {
    pub pk: String,
    pub sk: String,
}

impl From<SearchDocId> for DDBKey {
    fn from(id: SearchDocId) -> Self {
        DDBKey {
            pk: format!("document|{}", id.0),
            sk: format!("document|{}", id.0),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SearchDocId(String);

impl From<DDBKey> for SearchDocId {
    fn from(key: DDBKey) -> Self {
        let doc_id = key
            .pk
            .split("|")
            .nth(1)
            .expect("key should be formatted correctly");

        Self(doc_id.into())
    }
}

impl SearchDocId {
    pub fn parse(id: &str) -> SearchDocId {
        SearchDocId(id.into())
    }

    pub fn id(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchDoc {
    id: SearchDocId,
    content: Map<String, Value>,
}

impl SearchDoc {
    /// Converts a JSON value into a SearchDoc if the document is valid according to the schema.
    /// Also generate an `__id` if no `__id` is present.
    pub fn from_json(schema: &Schema, json_value: Value) -> Result<SearchDoc, SearchDocError> {
        let mut json_object = match json_value {
            Value::Object(obj) => obj,
            _ => return Err(SearchDocError::NotAnObject),
        };

        let id = json_object
            .entry("__id")
            .or_insert_with(|| json!(util::generate_id()))
            .as_str()
            .ok_or_else(|| SearchDocError::InvalidIdType)?
            .to_string();

        // Validate the document against the provided schema.
        let document = schema.json_object_to_doc(json_object.clone())?;

        if document.field_values().len() <= 1 {
            return Err(SearchDocError::EmptyDocument);
        }

        Ok(SearchDoc {
            id: SearchDocId(id),
            content: json_object,
        })
    }

    pub fn id(&self) -> &SearchDocId {
        &self.id
    }

    pub fn document(self, schema: &Schema) -> Document {
        schema
            .json_object_to_doc(self.content)
            .expect("should succeed since from_json validates")
    }
}

#[cfg(test)]
mod tests {
    use tantivy::schema;

    use super::*;

    fn setup() -> Schema {
        let mut schema = Schema::builder();
        schema.add_text_field("__id", schema::STRING);
        schema.add_text_field("name", schema::STRING);
        schema.build()
    }

    #[test]
    fn from_json_generates_id() {
        let schema = setup();
        let value = json!({
            "name": "world"
        });

        let search_doc = SearchDoc::from_json(&schema, value).unwrap();

        assert!(search_doc.id.0.len() > 0);
    }

    #[test]
    fn from_json_uses_id_when_exists() {
        let schema = setup();
        let id = util::generate_id();
        let value = json!({ "__id": id, "name": "world" });

        let search_doc = SearchDoc::from_json(&schema, value).unwrap();

        assert_eq!(id, search_doc.id.0);
    }

    #[test]
    fn from_json_returns_validation_error_when_schema_does_not_match() {
        let schema = setup();
        let value = json!({ "name": 1234 });

        let search_doc = SearchDoc::from_json(&schema, value).unwrap_err();

        assert_eq!(
            SearchDocError::SchemaValidationError(
                "The field '\"name\"' could not be parsed: TypeError { expected: \"a string\", \
                 json: Number(1234) }"
                    .into()
            ),
            search_doc,
        );
    }
}
