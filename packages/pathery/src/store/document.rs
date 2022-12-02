use std::fmt::Display;
use std::result::Result as StdResult;

use async_trait::async_trait;
use aws_sdk_dynamodb as ddb;
use ddb::error::{BatchGetItemError, BatchWriteItemError};
use ddb::model::{KeysAndAttributes, PutRequest, WriteRequest};
use ddb::types::SdkError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct DDBKey {
    pub pk: String,
    pub sk: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

impl From<SearchDocId> for DDBKey {
    fn from(id: SearchDocId) -> Self {
        DDBKey {
            pk: format!("document|{}", id.0),
            sk: format!("document|{}", id.0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDoc {
    pub id: SearchDocId,
    pub content: Vec<u8>,
}

#[derive(Debug, Error)]
enum DocumentStoreError {
    #[error("document store returned an incomplete response")]
    PartialResponse {
        documents: Vec<SearchDoc>,
        unprocessed_ids: Vec<SearchDocId>,
    },

    #[error("request exceeded the rate limit")]
    RequestRateLimitExceeded,

    #[error("unexpected error: [{reason:?}]")]
    UnexpectedError { reason: String },
}

impl<T> From<SdkError<T>> for DocumentStoreError
where
    SdkError<T>: Display,
    T: Into<DocumentStoreError>,
{
    fn from(sdk_err: SdkError<T>) -> Self {
        if let SdkError::ServiceError { err, .. } = sdk_err {
            err.into()
        } else {
            DocumentStoreError::UnexpectedError {
                reason: sdk_err.to_string(),
            }
        }
    }
}

impl From<BatchGetItemError> for DocumentStoreError {
    fn from(err: BatchGetItemError) -> Self {
        match err.kind {
            ddb::error::BatchGetItemErrorKind::RequestLimitExceeded(_) => {
                DocumentStoreError::RequestRateLimitExceeded
            }
            _ => DocumentStoreError::UnexpectedError {
                reason: err.to_string(),
            },
        }
    }
}

impl From<BatchWriteItemError> for DocumentStoreError {
    fn from(_sdk_err: BatchWriteItemError) -> Self {
        todo!()
    }
}

impl From<serde_dynamo::Error> for DocumentStoreError {
    fn from(err: serde_dynamo::Error) -> Self {
        DocumentStoreError::UnexpectedError {
            reason: err.to_string(),
        }
    }
}

type Result<T> = StdResult<T, DocumentStoreError>;

#[async_trait]
trait DocumentStore {
    /// Get a document by id.
    async fn get_documents(&self, ids: &[SearchDocId]) -> Result<Vec<SearchDoc>>;

    /// Save a document such that it can be retrieved with get_id.
    async fn save_documents(&self, documents: &[SearchDoc]) -> Result<()>;
}

pub struct DDBDocumentStore {
    table_name: String,
    client: ddb::Client,
}

#[async_trait]
impl DocumentStore for DDBDocumentStore {
    async fn get_documents(&self, ids: &[SearchDocId]) -> Result<Vec<SearchDoc>> {
        let mut request = self.client.batch_get_item();

        let mut keys_and_attrs = KeysAndAttributes::builder();

        for id in ids {
            let key = DDBKey::from(id.clone());
            keys_and_attrs = keys_and_attrs.keys(serde_dynamo::to_item(key)?);
        }

        request = request.request_items(&self.table_name, keys_and_attrs.build());

        let response = request.send().await?;

        let documents: StdResult<Vec<SearchDoc>, _> = response
            .responses()
            .expect("responses should be present")
            .values()
            .flatten()
            .map(|item| serde_dynamo::from_item(item.clone()))
            .collect();

        let documents = documents?;

        let unprocessed_ids: StdResult<Vec<_>, _> = response
            .unprocessed_keys()
            .expect("unprocessed keys should be present")
            .values()
            .filter_map(KeysAndAttributes::keys)
            .flatten()
            .map(|item| serde_dynamo::from_item(item.clone()))
            .collect();

        let unprocessed_ids = unprocessed_ids?;

        if unprocessed_ids.len() > 0 {
            return Err(DocumentStoreError::PartialResponse {
                documents,
                unprocessed_ids,
            });
        }

        Ok(documents)
    }

    async fn save_documents(&self, documents: &[SearchDoc]) -> Result<()> {
        let mut writes = vec![];

        for document in documents {
            let item = serde_dynamo::to_item(document)?;

            let put_request = PutRequest::builder().set_item(Some(item)).build();

            writes.push(WriteRequest::builder().put_request(put_request).build())
        }

        self.client
            .batch_write_item()
            .request_items(&self.table_name, writes)
            .send()
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn save_documents_to_store() {}
}