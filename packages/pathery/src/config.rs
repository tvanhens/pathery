use config::Config;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct CDKOutputs {
    #[serde(rename = "pathery-dev")]
    pathery_dev: PatheryDevOutputs,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct PatheryDevOutputs {
    #[serde(rename = "TestTableName")]
    table_name: String,
}

#[derive(Clone)]
pub struct AppConfig {
    dev_env_outputs: Option<CDKOutputs>,
}

impl AppConfig {
    pub fn load() -> AppConfig {
        let dev_env_outputs = Config::builder()
            .add_source(
                config::File::with_name("node_modules/@internal/dev-env/cdk-outputs.json")
                    .required(false),
            )
            .build()
            .and_then(|c| c.try_deserialize::<CDKOutputs>())
            .ok();

        AppConfig { dev_env_outputs }
    }

    pub fn table_name(&self) -> String {
        let from_env = std::env::var("TABLE_NAME").ok();
        let from_dev_env = self
            .dev_env_outputs
            .to_owned()
            .map(|o| o.pathery_dev.table_name);

        from_env.or(from_dev_env).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use anyhow::Result;
    use serde_json::json;

    #[test]
    fn test_parsing() -> Result<()> {
        let example_config = json!({
            "pathery-dev": {
                "TestTableName": "hello-table"
            }
        });

        let parsed = serde_json::from_value::<CDKOutputs>(example_config)?;

        assert_eq!(parsed.pathery_dev.table_name, "hello-table");

        Ok(())
    }

    // #[test]
    // fn test_table_from_env() {
    //     std::env::set_var("TABLE_NAME", "from-env");

    //     let config = AppConfig::load();

    //     assert_eq!(config.table_name(), "from-env");

    //     std::env::remove_var("TABLE_NAME");
    // }
}
