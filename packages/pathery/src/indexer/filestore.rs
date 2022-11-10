use std::sync::Arc;

use anyhow::Result;
use aws_sdk_dynamodb::{
    model::{AttributeValue, Put, TransactWriteItem},
    types::Blob,
};
use tokio::runtime::Runtime;

fn format_file_header_pk(store_id: &str) -> AttributeValue {
    AttributeValue::S(format!("store|{}|file_header", store_id))
}

fn format_file_content_pk(store_id: &str, path: &str) -> AttributeValue {
    AttributeValue::S(format!("store|{}|file_content|{}", store_id, path))
}

pub trait FileStore {
    fn write_file(&self, path: &str, content: &Vec<u8>) -> Result<()>;
    fn list_files(&self) -> Result<Vec<String>>;
    fn get_content(&self, path: &str) -> Result<Vec<u8>>;
}

#[derive(Debug)]
pub struct DynamoFileStore {
    table_name: String,
    store_id: String,
    client: Arc<aws_sdk_dynamodb::Client>,
    rt: Runtime,
}

impl DynamoFileStore {
    pub fn create(table_name: &str, store_id: &str) -> DynamoFileStore {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let config = rt.block_on(aws_config::load_from_env());
        let client = aws_sdk_dynamodb::Client::new(&config);

        DynamoFileStore {
            table_name: table_name.to_string(),
            store_id: store_id.to_string(),
            client: Arc::new(client.to_owned()),
            rt,
        }
    }
}

impl FileStore for DynamoFileStore {
    fn write_file(&self, path: &str, content: &Vec<u8>) -> Result<()> {
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
            .item("pk", content_item_key.to_owned())
            .item("sk", content_item_key.to_owned())
            .item("store_id", AttributeValue::S(self.store_id.to_string()))
            .item("content", AttributeValue::B(Blob::new(content.to_owned())))
            .build();

        self.rt.block_on(
            self.client
                .transact_write_items()
                .transact_items(TransactWriteItem::builder().put(header_item).build())
                .transact_items(TransactWriteItem::builder().put(content_item).build())
                .send(),
        )?;

        Ok(())
    }

    fn list_files(&self) -> Result<Vec<String>> {
        let response = self.rt.block_on(
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
        let response = self.rt.block_on(
            self.client
                .get_item()
                .table_name(&self.table_name)
                .key("pk", key.to_owned())
                .key("sk", key.to_owned())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn write_and_read_file() -> Result<()> {
        let store_id = Uuid::new_v4();
        let store = DynamoFileStore::create(
            "pathery-dev-TestTable5769773A-CC2G78O275F1",
            &store_id.to_string(),
        );
        let content = "hello world!".as_bytes().to_vec();

        store.write_file("hello.txt", &content)?;

        let files = store.list_files()?;

        assert_eq!(vec!["hello.txt"], files);

        let read_content = store.get_content(files.get(0).unwrap())?;

        assert_eq!(content, read_content);

        Ok(())
    }
}
