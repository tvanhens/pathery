use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tantivy::schema::{self, Schema, TextOptions};

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

pub struct IndexLoader {
    configs: Vec<IndexConfig>,
}

impl IndexLoader {
    pub fn create(root_path: &str) -> Result<IndexLoader> {
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

        Ok(IndexLoader { configs })
    }

    pub fn lambda() -> Result<IndexLoader> {
        IndexLoader::create("/opt/pathery-config")
    }

    pub fn schema_for(&self, index_id: &str) -> Option<Schema> {
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

                schema.build()
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;

    #[test]
    fn parse_index_config() -> Result<()> {
        let parsed: IndexConfig = serde_yaml::from_str(
            "
        prefix: hello
        fields:
            - name: stuff
              kind: !Text { options: [STORED] }
            - name: things
              kind: !Text { options: [] }
              ",
        )?;

        assert_eq!(parsed.fields.len(), 2);

        Ok(())
    }

    #[test]
    fn load_test_config() -> Result<()> {
        let loader = IndexLoader::create("../../app/config/pathery-config")?;
        loader.schema_for("book-index-1").unwrap();
        Ok(())
    }
}
