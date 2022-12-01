use std::fs;

use serde::{Deserialize, Serialize};
use serde_json as json;
use tantivy::schema::{self, DocParsingError, Field, NumericOptions, Schema, TextOptions};
use tantivy::Document;
use thiserror::Error;

use crate::util;

#[derive(Serialize, Deserialize, Debug)]
pub enum TextFieldOption {
    STORED,
    TEXT,
    STRING,
    FAST,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NumericFieldOption {
    STORED,
    INDEXED,
    FAST,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum FieldConfig {
    #[serde(rename = "text")]
    TextFieldConfig {
        name: String,
        flags: Vec<TextFieldOption>,
    },
    #[serde(rename = "date")]
    DateFieldConfig {
        name: String,
        flags: Vec<NumericFieldOption>,
    },
    #[serde(rename = "i64")]
    IntegerFieldConfig {
        name: String,
        flags: Vec<NumericFieldOption>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexConfig {
    prefix: String,
    fields: Vec<FieldConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatheryConfig {
    indexes: Vec<IndexConfig>,
}

pub trait SchemaLoader: Send + Sync {
    fn load_schema(&self, index_id: &str) -> Schema;
}

#[derive(Error, Debug)]
pub enum IndexDocError {
    #[error("Expected JSON object")]
    NotJsonObject,
    #[error("Request JSON object is empty")]
    EmptyDoc,
    #[error("Error parsing JSON object document")]
    DocParsingError(DocParsingError),
}

fn numeric_field_options(flags: &Vec<NumericFieldOption>) -> NumericOptions {
    flags
        .iter()
        .fold(NumericOptions::default(), |acc, opt| match opt {
            NumericFieldOption::STORED => acc | schema::STORED,
            NumericFieldOption::INDEXED => acc | schema::INDEXED,
            NumericFieldOption::FAST => acc | schema::FAST,
        })
}

pub trait SchemaExt {
    fn id_field(&self) -> Field;

    fn to_document(&self, json_doc: json::Value) -> Result<(String, Document), IndexDocError>;
}

impl SchemaExt for Schema {
    fn id_field(&self) -> Field {
        self.get_field("__id")
            .expect("__id field should be present")
    }

    fn to_document(&self, json_doc: json::Value) -> Result<(String, Document), IndexDocError> {
        let json_doc = if let json::Value::Object(obj) = json_doc {
            obj
        } else {
            return Err(IndexDocError::NotJsonObject);
        };

        let doc_id = json_doc
            .get("__id")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut document = self
            .json_object_to_doc(json_doc)
            .map_err(IndexDocError::DocParsingError)?;

        if document.is_empty() {
            return Err(IndexDocError::EmptyDoc);
        }

        match doc_id {
            Some(doc_id) => Ok((doc_id, document)),
            None => {
                let id_field = self.id_field();
                let doc_id = util::generate_id();
                document.add_text(id_field, &doc_id);
                Ok((doc_id, document))
            }
        }
    }
}

pub struct SchemaProvider {
    config: PatheryConfig,
}

impl SchemaProvider {
    pub fn lambda() -> Self {
        let config_path = "/opt/pathery/config.json";
        let content = fs::read_to_string(config_path).expect("config should exist");
        let config: PatheryConfig = json::from_str(&content).expect("config should parse");

        SchemaProvider { config }
    }

    pub fn from_json(config: json::Value) -> Self {
        let config = json::from_value(config).expect("config should parse");
        Self { config }
    }
}

impl SchemaLoader for SchemaProvider {
    fn load_schema(&self, index_id: &str) -> Schema {
        let config = self
            .config
            .indexes
            .iter()
            .find(|config| index_id.starts_with(&config.prefix))
            .expect("schema definition should exist");

        let mut schema = Schema::builder();

        for field in &config.fields {
            match &field {
                FieldConfig::TextFieldConfig { name, flags } => {
                    let field_opts =
                        flags
                            .iter()
                            .fold(TextOptions::default(), |acc, opt| match opt {
                                TextFieldOption::TEXT => acc | schema::TEXT,
                                TextFieldOption::STORED => acc | schema::STORED,
                                TextFieldOption::STRING => acc | schema::STRING,
                                TextFieldOption::FAST => acc | schema::FAST,
                            });
                    schema.add_text_field(name, field_opts);
                }
                FieldConfig::DateFieldConfig { name, flags } => {
                    schema.add_date_field(name, numeric_field_options(flags));
                }
                FieldConfig::IntegerFieldConfig { name, flags } => {
                    schema.add_i64_field(name, numeric_field_options(flags));
                }
            }
        }

        // Add system schema fields

        // __id is the document id used for uniqueness
        schema.add_text_field("__id", schema::STRING | schema::STORED);

        schema.build()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_test_config() {
        let config = json!({
                "indexes": [{
                    "prefix": "book-index-v1-",
                    "fields": [
                        {
                            "name": "title",
                            "flags": ["STORED", "TEXT"],
                            "kind": "text",
                        },
                        {
                            "name": "author",
                            "flags": ["STORED", "STRING"],
                            "kind": "text",
                        },
                        {
                            "name": "date_added",
                            "flags": ["STORED", "INDEXED", "FAST"],
                            "kind": "date",
                        },
                        {
                            "name": "year",
                            "flags": ["STORED", "INDEXED", "FAST"],
                            "kind": "i64",
                        },
                    ],
            }]
        });

        serde_json::from_value::<PatheryConfig>(config).expect("should not throw");
    }

    #[test]
    fn serialize_schema() {
        let mut schema = Schema::builder();

        schema.add_text_field("title", schema::STORED | schema::TEXT);
        schema.add_text_field("author", schema::STORED | schema::STRING);
        schema.add_date_field(
            "created_date",
            schema::STORED | schema::INDEXED | schema::FAST,
        );

        let schema = schema.build();

        println!("{}", json::to_string_pretty(&schema).expect("ok"));
    }
}
