use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
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
pub struct FieldConfig {
    name: String,
    kind: FieldKindConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexConfig {
    prefix: String,
    fields: Vec<FieldConfig>,
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
    configs: Vec<IndexConfig>,
}

impl DirSchemaLoader {
    pub fn create() -> Result<Self> {
        let root_path = "/opt/pathery-config";
        let files: Vec<PathBuf> = fs::read_dir(root_path)?
            .into_iter()
            .map(|entry| entry.unwrap().path())
            .collect();
        let mut configs: Vec<IndexConfig> = Vec::new();

        for p in files {
            let content = fs::read_to_string(p)?;
            let config: IndexConfig = serde_yaml::from_str(&content)?;
            configs.push(config);
        }

        Ok(DirSchemaLoader { configs })
    }
}

impl SchemaLoader for DirSchemaLoader {
    fn load_schema(&self, index_id: &str) -> Schema {
        self.configs
            .iter()
            .find(|config| index_id.starts_with(&config.prefix))
            .map(|config| {
                let mut schema = Schema::builder();

                for field in &config.fields {
                    match &field.kind {
                        FieldKindConfig::Text { options } => {
                            let field_opts =
                                options
                                    .iter()
                                    .fold(TextOptions::default(), |acc, opt| match opt {
                                        TextFieldOption::TEXT => acc | schema::TEXT,
                                        TextFieldOption::STORED => acc | schema::STORED,
                                    });
                            schema.add_text_field(&field.name, field_opts);
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
