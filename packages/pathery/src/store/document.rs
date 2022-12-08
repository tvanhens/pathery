use std::collections::HashMap;
use std::fmt::Display;
use std::result::Result as StdResult;

use async_trait::async_trait;
use aws_sdk_dynamodb as ddb;
use ddb::model::{AttributeValue, KeysAndAttributes, PutRequest, WriteRequest};
use ddb::types::SdkError;
use serde::{Deserialize, Serialize};
use tantivy::schema::NamedFieldDocument;

use crate::search_doc::{DDBKey, SearchDoc, SearchDocId};
use crate::service::ServiceError;
use crate::util;

impl<T> From<SdkError<T>> for ServiceError
where SdkError<T>: Display
{
    fn from(sdk_err: SdkError<T>) -> Self {
        ServiceError::InternalError(sdk_err.to_string())
    }
}

impl From<serde_dynamo::Error> for ServiceError {
    fn from(err: serde_dynamo::Error) -> Self {
        ServiceError::internal_error(&err.to_string())
    }
}

type Result<T> = StdResult<T, ServiceError>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SearchDocRef(SearchDocId);

impl From<NamedFieldDocument> for SearchDocRef {
    fn from(doc: NamedFieldDocument) -> Self {
        let id = doc
            .0
            .get("__id")
            .expect("__id should be set")
            .first()
            .expect("__id should exist")
            .as_text()
            .expect("__id should be string");

        SearchDocRef(SearchDocId::parse(id))
    }
}

#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// Get documents by reference.
    async fn get_documents(&self, refs: Vec<SearchDocRef>) -> Result<Vec<SearchDoc>>;

    /// Save a document such that it can be retrieved with get_documents.
    async fn save_documents(&self, documents: Vec<SearchDoc>) -> Result<Vec<SearchDocRef>>;
}

pub struct DDBDocumentStore {
    table_name: String,
    client: ddb::Client,
}

#[async_trait]
impl DocumentStore for DDBDocumentStore {
    async fn get_documents(&self, refs: Vec<SearchDocRef>) -> Result<Vec<SearchDoc>> {
        let mut request = self.client.batch_get_item();

        let mut keys_and_attrs = KeysAndAttributes::builder();

        for doc_ref in refs {
            let key = DDBKey::from(doc_ref.0);
            keys_and_attrs = keys_and_attrs.keys(serde_dynamo::to_item(key)?);
        }

        request = request.request_items(&self.table_name, keys_and_attrs.build());

        let response = request.send().await?;

        let documents = response
            .responses()
            .expect("responses should be present")
            .values()
            .flatten()
            .map(|item| serde_dynamo::from_item(item.clone()))
            .collect::<StdResult<Vec<SearchDoc>, _>>()?;

        let unprocessed_ids = response
            .unprocessed_keys()
            .expect("unprocessed keys should be present")
            .values()
            .filter_map(KeysAndAttributes::keys)
            .flatten()
            .collect::<Vec<_>>();

        if unprocessed_ids.len() > 0 {
            return Err(ServiceError::rate_limit());
        }

        Ok(documents)
    }

    async fn save_documents(&self, documents: Vec<SearchDoc>) -> Result<Vec<SearchDocRef>> {
        let mut writes = vec![];

        for document in &documents {
            let mut item: HashMap<String, AttributeValue> = serde_dynamo::to_item(document)?;

            let key: HashMap<String, AttributeValue> =
                serde_dynamo::to_item(DDBKey::from(document.id().clone()))?;

            item.extend(key);

            let put_request = PutRequest::builder().set_item(Some(item)).build();

            writes.push(WriteRequest::builder().put_request(put_request).build())
        }

        let response = self
            .client
            .batch_write_item()
            .request_items(&self.table_name, writes)
            .send()
            .await?;

        if let Some(items) = response.unprocessed_items() {
            let unhandled_writes = items.values().flatten().collect::<Vec<_>>();
            if unhandled_writes.len() > 0 {
                return Err(ServiceError::rate_limit());
            }
        };

        Ok(documents
            .into_iter()
            .map(|doc| SearchDocRef(doc.id().clone()))
            .collect())
    }
}

impl DDBDocumentStore {
    pub async fn create(table_name: Option<&str>) -> DDBDocumentStore {
        let table_name = table_name
            .map(String::from)
            .unwrap_or_else(|| util::require_env("DATA_TABLE_NAME"));
        let sdk_config = aws_config::load_from_env().await;
        let client = aws_sdk_dynamodb::Client::new(&sdk_config);

        DDBDocumentStore { table_name, client }
    }
}

#[cfg(test)]
pub mod test_util {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Debug)]
    pub struct TestDocumentStore {
        db: Arc<Mutex<HashMap<SearchDocId, SearchDoc>>>,
    }

    #[async_trait]
    impl DocumentStore for TestDocumentStore {
        async fn save_documents(&self, documents: Vec<SearchDoc>) -> Result<Vec<SearchDocRef>> {
            let mut db = self.db.lock().unwrap();

            for document in &documents {
                (*db).insert(document.id().clone(), document.clone());
            }

            Ok(documents
                .iter()
                .map(|x| SearchDocRef(x.id().clone()))
                .collect())
        }

        async fn get_documents(&self, refs: Vec<SearchDocRef>) -> Result<Vec<SearchDoc>> {
            let db = self.db.lock().unwrap();

            Ok(refs
                .iter()
                .map(|doc_ref| (*db).get(&doc_ref.0).unwrap().clone())
                .collect())
        }
    }

    impl TestDocumentStore {
        pub fn create() -> Self {
            TestDocumentStore {
                db: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }
}
