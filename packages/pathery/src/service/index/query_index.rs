use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, FieldType};
use tantivy::{DocAddress, Score, SnippetGenerator, TantivyError};
use tracing::info;

use crate::index::{IndexLoader, LambdaIndexLoader};
use crate::json;
use crate::service::{ServiceError, ServiceHandler, ServiceRequest, ServiceResponse};
use crate::store::document::{DDBDocumentStore, DocumentStore, SearchDocRef};

#[derive(Serialize, Deserialize, Debug)]
pub struct WithPartition {
    partition_n: usize,

    total_partitions: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRequest {
    pub query: String,

    pub with_partition: Option<WithPartition>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SearchHit {
    pub doc: json::Value,
    pub snippets: json::Value,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct QueryResponse {
    pub matches: Vec<SearchHit>,
}

pub struct QueryIndexService {
    index_loader: Box<dyn IndexLoader>,

    document_store: Box<dyn DocumentStore>,
}

#[async_trait]
impl ServiceHandler<QueryRequest, QueryResponse> for QueryIndexService {
    async fn handle_request(
        &self,
        request: ServiceRequest<QueryRequest>,
    ) -> ServiceResponse<QueryResponse> {
        let body = request.body()?;

        let index_id = request.path_param("index_id")?;

        let index = self.index_loader.load_index(
            &index_id,
            body.with_partition
                .map(|x| (x.partition_n, x.total_partitions)),
        )?;

        let reader = index.reader().expect("Reader should load");

        info!("ReaderLoaded");

        let searcher = reader.searcher();

        let schema = index.schema();

        let query_parser = QueryParser::for_index(
            &index,
            schema
                .fields()
                .filter_map(|(field, entry)| {
                    if !entry.is_indexed() {
                        return None;
                    }
                    match entry.field_type() {
                        FieldType::Str(_) => Some(field),
                        _ => None,
                    }
                })
                .collect::<Vec<Field>>(),
        );

        let query = query_parser
            .parse_query(&body.query)
            .map_err(|err| ServiceError::invalid_request(&err.to_string()))?;

        let top_docs: Vec<(Score, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(10))
            .expect("search should succeed");

        let matches: Vec<_> = top_docs
            .into_iter()
            .map(|(score, address)| {
                let document = searcher.doc(address).expect("doc should exist");

                let named_doc = schema.to_named_doc(&document);

                let stored_ref = SearchDocRef::from(named_doc);

                (score, stored_ref)
            })
            .collect();

        if matches.len() == 0 {
            return Ok(QueryResponse { matches: vec![] });
        }

        let retrieved_matches = self
            .document_store
            .get_documents(
                matches
                    .iter()
                    .map(|(_score, doc_ref)| doc_ref.clone())
                    .collect(),
            )
            .await
            .unwrap();

        let matches = retrieved_matches
            .iter()
            .zip(matches)
            .map(|(search_doc, (score, _))| {
                let document = search_doc.document(&schema);

                let named_doc = schema.to_named_doc(&document);

                let snippets: HashMap<String, String> = document
                    .field_values()
                    .iter()
                    .filter_map(|field_value| {
                        // Only text fields are supported for snippets
                        let text = field_value.value().as_text()?;

                        let generator = match SnippetGenerator::create(
                            &searcher,
                            &query,
                            field_value.field(),
                        ) {
                            Ok(generator) => Some(generator),
                            // InvalidArgument is returned when field is not indexed
                            Err(TantivyError::InvalidArgument(_)) => None,
                            Err(err) => panic!("{}", err.to_string()),
                        }?;

                        let snippet = generator.snippet(text).to_html();

                        if snippet.is_empty() {
                            None
                        } else {
                            Some((schema.get_field_name(field_value.field()).into(), snippet))
                        }
                    })
                    .collect();

                SearchHit {
                    score,
                    doc: json::to_value(named_doc).expect("named doc should serialize"),
                    snippets: json::to_value(snippets).expect("snippets should serialize"),
                }
            })
            .collect();

        Ok(QueryResponse { matches })
    }
}

impl QueryIndexService {
    pub async fn create() -> QueryIndexService {
        let document_store = DDBDocumentStore::create(None).await;
        let index_loader = LambdaIndexLoader::create();

        QueryIndexService {
            document_store: Box::new(document_store),
            index_loader: Box::new(index_loader),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    fn test_service(ctx: &TestContext) -> QueryIndexService {
        QueryIndexService {
            document_store: Box::new(ctx.document_store().clone()),
            index_loader: Box::new(ctx.index_loader().clone()),
        }
    }

    #[tokio::test]
    async fn query_default_response() {
        let ctx = setup()
            .with_documents(
                "test",
                vec![json!({
                    "__id": "foobar",
                    "title": "hello",
                    "author": "world"
                })],
            )
            .await;

        let service = test_service(&ctx);

        let request = ServiceRequest::create(QueryRequest {
            query: "hello".into(),
            with_partition: None,
        })
        .with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap();

        assert_eq!(
            QueryResponse {
                matches: vec![SearchHit {
                    doc: json::json!({
                        "__id": ["foobar"],
                        "title": ["hello"],
                        "author": ["world"],
                    }),
                    score: 0.28768212,
                    snippets: json::json!({
                        "title": "<b>hello</b>"
                    })
                }]
            },
            response
        );
    }

    #[tokio::test]
    async fn query_document_with_un_indexed_fields() {
        let ctx = setup()
            .with_documents(
                "test",
                vec![json!({
                    "__id": "foobar",
                    "title": "hello",
                    "meta": "world"
                })],
            )
            .await;

        let service = test_service(&ctx);

        let request = ServiceRequest::create(QueryRequest {
            query: "hello".into(),
            with_partition: None,
        })
        .with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap();

        assert_eq!(1, response.matches.len());
    }

    #[tokio::test]
    async fn query_document_with_json_field() {
        let ctx = setup()
            .with_documents(
                "test",
                vec![json!({
                    "__id": "foobar",
                    "title": "hello",
                    "props": {
                        "foo": "bar"
                    }
                })],
            )
            .await;

        let service = test_service(&ctx);

        let request = ServiceRequest::create(QueryRequest {
            query: "props.foo:bar".into(),
            with_partition: None,
        })
        .with_path_param("index_id", "test");

        let response = service.handle_request(request).await.unwrap();

        assert_eq!(1, response.matches.len());
    }
}
