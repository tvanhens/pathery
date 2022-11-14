use std::sync::Arc;

use anyhow::Result;
use aws_sdk_dynamodb::{
    model::{AttributeValue, Put, TransactWriteItem},
    types::Blob,
    Client as DDBClient,
};
use tokio::runtime::Handle;

fn format_file_header_pk(store_id: &str) -> AttributeValue {
    AttributeValue::S(format!("store|{}|file_header", store_id))
}

fn format_file_content_pk(store_id: &str, path: &str) -> AttributeValue {
    AttributeValue::S(format!("store|{}|file_content|{}", store_id, path))
}

pub trait FileStore {
    fn delete(&self, path: &str) -> Result<()>;
    fn exists(&self, path: &str) -> Result<bool>;
    fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;
    fn list_files(&self) -> Result<Vec<String>>;
    fn get_content(&self, path: &str) -> Result<Vec<u8>>;
}

#[derive(Debug)]
pub struct DynamoFileStore {
    table_name: String,
    store_id: String,
    client: Arc<DDBClient>,
    handle: Handle,
}

impl DynamoFileStore {
    pub fn create(client: &Arc<DDBClient>, table_name: &str, store_id: &str) -> DynamoFileStore {
        let handle = Handle::try_current().expect("Tokio runtime should be present but not found.");

        DynamoFileStore {
            table_name: table_name.to_string(),
            store_id: store_id.to_string(),
            client: Arc::clone(client),
            handle,
        }
    }
}

impl FileStore for DynamoFileStore {
    fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let header_item = Put::builder()
            .table_name(&self.table_name)
            .item("pk", format_file_header_pk(&self.store_id))
            .item("sk", AttributeValue::S(format!("file_header|{}", path)))
            .item("store_id", AttributeValue::S(self.store_id.to_string()))
            .item("path", AttributeValue::S(path.to_string()))
            .build();

        let content_item_key = format_file_content_pk(&self.store_id, path);
        let content_item = Put::builder()
            .table_name(&self.table_name)
            .item("pk", content_item_key.clone())
            .item("sk", content_item_key)
            .item("store_id", AttributeValue::S(self.store_id.to_string()))
            .item("content", AttributeValue::B(Blob::new(content.to_owned())))
            .build();

        self.handle.block_on(
            self.client
                .transact_write_items()
                .transact_items(TransactWriteItem::builder().put(header_item).build())
                .transact_items(TransactWriteItem::builder().put(content_item).build())
                .send(),
        )?;

        Ok(())
    }

    fn exists(&self, path: &str) -> Result<bool> {
        match self
            .handle
            .block_on(
                self.client
                    .get_item()
                    .table_name(&self.table_name)
                    .key("pk", format_file_header_pk(&self.store_id))
                    .key("sk", AttributeValue::S(format!("file_header|{}", path)))
                    .consistent_read(true)
                    .send(),
            )
            .unwrap()
            .item()
        {
            Some(_item) => Ok(true),
            None => Ok(false),
        }
    }

    fn list_files(&self) -> Result<Vec<String>> {
        let response = self.handle.block_on(
            self.client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("#pk = :pk")
                .expression_attribute_names("#pk", "pk")
                .expression_attribute_values(":pk", format_file_header_pk(&self.store_id))
                .send(),
        )?;

        Ok(response
            .items()
            .unwrap()
            .iter()
            .map(|item| item.get("path").unwrap().as_s().unwrap().to_string())
            .collect())
    }

    fn get_content(&self, path: &str) -> Result<Vec<u8>> {
        let key = format_file_content_pk(&self.store_id, path);
        let response = self.handle.block_on(
            self.client
                .get_item()
                .table_name(&self.table_name)
                .key("pk", key.clone())
                .key("sk", key)
                .send(),
        )?;

        if let Some(item) = response.item() {
            Ok(item
                .get("content")
                .unwrap()
                .as_b()
                .unwrap()
                .clone()
                .into_inner())
        } else {
            Ok(Vec::new())
        }
    }

    fn delete(&self, path: &str) -> Result<()> {
        self.handle.block_on(
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("pk", format_file_header_pk(&self.store_id))
                .key("sk", AttributeValue::S(format!("file_header|{}", path)))
                .send(),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    use super::*;
    use tokio::task;
    use uuid::Uuid;

    async fn test_store() -> DynamoFileStore {
        std::env::set_var("AWS_PROFILE", "pathery-dev");
        let sdk_config = aws_config::load_from_env().await;
        let client = Arc::new(DDBClient::new(&sdk_config));
        let config = AppConfig::load();
        let store_id = Uuid::new_v4();
        DynamoFileStore::create(&client, &config.table_name(), &store_id.to_string())
    }

    #[tokio::test]
    async fn write_and_read_file() -> Result<()> {
        let store = test_store().await;

        task::spawn_blocking(move || {
            assert_eq!(false, store.exists("hello.txt")?);

            let content = "hello world!".as_bytes().to_vec();

            store.write_file("hello.txt", &content)?;

            assert_eq!(true, store.exists("hello.txt")?);

            let files = store.list_files()?;

            assert_eq!(vec!["hello.txt"], files);

            let read_content = store.get_content(files.get(0).unwrap())?;

            assert_eq!(content, read_content);

            store.delete("hello.txt")?;

            assert_eq!(false, store.exists("hello.txt")?);

            Ok(())
        })
        .await
        .unwrap()
    }
}
