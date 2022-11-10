use std::collections::HashMap;

use anyhow::Result;
use aws_sdk_dynamodb::{
    model::AttributeValue,
    output::{PutItemOutput, QueryOutput},
};
use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct DynamoTable {
    table_name: String,
    client: aws_sdk_dynamodb::Client,
    rt: Runtime,
}

impl DynamoTable {
    pub fn new(table_name: String) -> Result<DynamoTable> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let config = rt.block_on(aws_config::load_from_env());

        Ok(DynamoTable {
            table_name,
            client: aws_sdk_dynamodb::Client::new(&config),
            rt,
        })
    }

    async fn put_item(&self, item: HashMap<String, AttributeValue>) -> Result<PutItemOutput> {
        Ok(self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?)
    }

    fn put_item_sync(&self, item: HashMap<String, AttributeValue>) -> Result<PutItemOutput> {
        self.rt.block_on(self.put_item(item))
    }

    async fn list(&self, pk: &str) -> Result<QueryOutput> {
        Ok(self
            .client
            .query()
            .table_name(&self.table_name)
            .consistent_read(true)
            .key_condition_expression("#pk = :pk")
            .expression_attribute_names("#pk", "pk")
            .expression_attribute_values(":pk", AttributeValue::S(pk.to_string()))
            .send()
            .await?)
    }

    fn list_sync(&self, pk: &str) -> Result<QueryOutput> {
        self.rt.block_on(self.list(pk))
    }
}

pub trait DynamoRecord<PK> {
    fn serialize(&self) -> HashMap<String, AttributeValue>;

    fn deserialize(item: &HashMap<String, AttributeValue>) -> Self;

    fn format_pk(pk: PK) -> String;

    fn save(&self, table: &DynamoTable) -> Result<()> {
        table.put_item_sync(self.serialize())?;
        Ok(())
    }

    fn list(table: &DynamoTable, pk: PK) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        Ok(table
            .list_sync(&Self::format_pk(pk))?
            .items()
            .unwrap()
            .iter()
            .map(DynamoRecord::deserialize)
            .collect::<Vec<Self>>())
    }
}
