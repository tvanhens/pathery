use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json as json;
use std::fs;
use tantivy::schema::{self, Field, Schema, TextOptions};

#[derive(Serialize, Deserialize, Debug)]
pub enum TextFieldOption {
    STORED,
    TEXT,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FieldKindConfig {
    Text { options: Vec<TextFieldOption> },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum FieldConfig {
    #[serde(rename = "text")]
    TextFieldConfig {
        name: String,
        flags: Vec<TextFieldOption>,
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

pub trait SchemaLoader {
    fn load_schema(&self, index_id: &str) -> Schema;
}

pub trait TantivySchema {
    fn id_field(&self) -> Field;
}

impl TantivySchema for Schema {
    fn id_field(&self) -> Field {
        self.get_field("__id")
            .expect("__id field should be present")
    }
}

pub struct DirSchemaLoader {
    config: PatheryConfig,
}

impl DirSchemaLoader {
    pub fn create() -> Result<Self> {
        let config_path = "/opt/pathery/config.json";
        let content = fs::read_to_string(config_path)?;
        let config: PatheryConfig = json::from_str(&content)?;

        Ok(DirSchemaLoader { config })
    }
}

impl SchemaLoader for DirSchemaLoader {
    fn load_schema(&self, index_id: &str) -> Schema {
        self.config
            .indexes
            .iter()
            .find(|config| index_id.starts_with(&config.prefix))
            .map(|config| {
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
                                    });
                            schema.add_text_field(name, field_opts);
                        }
                    }
                }

                // Add system schema fields

                // __id is the document id used for uniqueness
                schema.add_text_field("__id", schema::STRING | schema::STORED);

                schema.build()
            })
            .expect("schema definition should exist")
    }
}

pub struct TestSchemaLoader {}

impl SchemaLoader for TestSchemaLoader {
    fn load_schema(&self, _index_id: &str) -> Schema {
        let mut schema = Schema::builder();

        schema.add_text_field("__id", schema::STRING | schema::STORED);
        schema.add_text_field("author", schema::TEXT | schema::STORED);
        schema.add_text_field("title", schema::TEXT | schema::STORED);
        schema.add_text_field("body", schema::TEXT | schema::STORED);

        schema.build()
    }
}

pub fn test_schema_loader() -> TestSchemaLoader {
    TestSchemaLoader {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
                        "flags": ["STORED", "TEXT"],
                        "kind": "text",
                        },
                    ],
            }]
        });

        serde_json::from_value::<PatheryConfig>(config).expect("should not throw");
    }
}
