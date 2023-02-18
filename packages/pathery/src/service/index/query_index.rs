use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tantivy::query::QueryParser;
use tantivy::schema::{Field, FieldType};
use tantivy::{Index, SnippetGenerator, TantivyError};
use tracing::info;

use crate::function::query_index_partition::client::LambdaQueryIndexPartitionClient;
use crate::function::query_index_partition::PartitionSearchHit;
use crate::index::{IndexExt, IndexLoader, LambdaIndexLoader};
use crate::json;
use crate::service::{ServiceError, ServiceHandler, ServiceRequest, ServiceResponse};
use crate::store::document::{DDBDocumentStore, DocumentStore};

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRequest {
    pub query: String,
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

    query_index_paritition_client: LambdaQueryIndexPartitionClient,
}

#[async_trait]
impl ServiceHandler<QueryRequest, QueryResponse> for QueryIndexService {
    async fn handle_request(
        &self,
        request: ServiceRequest<QueryRequest>,
    ) -> ServiceResponse<QueryResponse> {
        let body = request.body()?;

        let index_id = request.path_param("index_id")?;

        let index = self.index_loader.load_index(&index_id, None)?;

        let metas = index.load_metas().unwrap();
        let num_docs: u32 = metas.segments.iter().map(|seg| seg.num_docs()).sum();
        info!("Doc count: {}", num_docs);

        let total_partitions = (num_docs / 60_000) + 1;
        info!("Total partitions: {}", total_partitions);

        let requests = (0..total_partitions).map(|partition_n| {
            self.query_index_paritition_client.query_partition(
                index_id.clone(),
                body.query.clone(),
                total_partitions as usize,
                partition_n as usize,
            )
        });

        let mut matches: Vec<PartitionSearchHit> = Vec::new();

        for request in requests {
            let mut response = request.await;
            let response = response.matches.as_mut();
            matches.append(response);
        }

        matches.sort_by(|a, b| b.score.total_cmp(&a.score));
        matches.truncate(10);

        if matches.len() == 0 {
            return Ok(QueryResponse { matches: vec![] });
        }

        let retrieved_matches = self
            .document_store
            .get_documents(
                matches
                    .iter()
                    .map(|one_match| one_match.doc_ref.clone())
                    .collect(),
            )
            .await
            .unwrap();

        let snippet_index = Index::create_in_ram(index.schema());
        let mut snippet_writer = snippet_index.default_writer();
        let snippet_reader = snippet_index.reader().unwrap();
        let snippet_schema = snippet_index.schema();

        let query_parser = QueryParser::for_index(
            &snippet_index,
            snippet_schema
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

        let matches = retrieved_matches
            .iter()
            .zip(matches)
            .map(|(search_doc, one_match)| {
                let document = search_doc.document(&snippet_schema);
                let named_doc = snippet_schema.to_named_doc(&document);
                snippet_writer.add_document(document.clone()).unwrap();
                snippet_writer.commit().unwrap();
                snippet_reader.reload().unwrap();
                let snippet_searcher = snippet_reader.searcher();

                let snippets: HashMap<String, String> = document
                    .field_values()
                    .iter()
                    .filter_map(|field_value| {
                        // Only text fields are supported for snippets
                        let text = field_value.value().as_text()?;

                        let generator = match SnippetGenerator::create(
                            &snippet_searcher,
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
                            Some((
                                snippet_schema.get_field_name(field_value.field()).into(),
                                snippet,
                            ))
                        }
                    })
                    .collect();

                SearchHit {
                    score: one_match.score,
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
            index_loader: Box::new(index_loader.await),
            query_index_paritition_client: LambdaQueryIndexPartitionClient::create().await,
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::test_utils::*;

//     fn test_service(ctx: &TestContext) -> QueryIndexService {
//         QueryIndexService {
//             document_store: Box::new(ctx.document_store().clone()),
//             index_loader: Box::new(ctx.index_loader().clone()),
//         }
//     }

//     #[tokio::test]
//     async fn query_default_response() {
//         let ctx = setup()
//             .with_documents(
//                 "test",
//                 vec![json!({
//                     "__id": "foobar",
//                     "title": "hello",
//                     "author": "world"
//                 })],
//             )
//             .await;

//         let service = test_service(&ctx);

//         let request = ServiceRequest::create(QueryRequest {
//             query: "hello".into(),
//             with_partition: None,
//         })
//         .with_path_param("index_id", "test");

//         let response = service.handle_request(request).await.unwrap();

//         assert_eq!(
//             QueryResponse {
//                 matches: vec![SearchHit {
//                     doc: json::json!({
//                         "__id": ["foobar"],
//                         "title": ["hello"],
//                         "author": ["world"],
//                     }),
//                     score: 0.28768212,
//                     snippets: json::json!({
//                         "title": "<b>hello</b>"
//                     })
//                 }]
//             },
//             response
//         );
//     }

//     #[tokio::test]
//     async fn query_document_with_un_indexed_fields() {
//         let ctx = setup()
//             .with_documents(
//                 "test",
//                 vec![json!({
//                     "__id": "foobar",
//                     "title": "hello",
//                     "meta": "world"
//                 })],
//             )
//             .await;

//         let service = test_service(&ctx);

//         let request = ServiceRequest::create(QueryRequest {
//             query: "hello".into(),
//             with_partition: None,
//         })
//         .with_path_param("index_id", "test");

//         let response = service.handle_request(request).await.unwrap();

//         assert_eq!(1, response.matches.len());
//     }

//     #[tokio::test]
//     async fn query_document_with_json_field() {
//         let ctx = setup()
//             .with_documents(
//                 "test",
//                 vec![json!({
//                     "__id": "foobar",
//                     "title": "hello",
//                     "props": {
//                         "foo": "bar"
//                     }
//                 })],
//             )
//             .await;

//         let service = test_service(&ctx);

//         let request = ServiceRequest::create(QueryRequest {
//             query: "props.foo:bar".into(),
//             with_partition: None,
//         })
//         .with_path_param("index_id", "test");

//         let response = service.handle_request(request).await.unwrap();

//         assert_eq!(1, response.matches.len());
//     }
// }
