pub mod client;

use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, FieldType};
use tantivy::{DocAddress, Score};

use crate::index::IndexLoader;
use crate::pagination::SegmentMeta;
use crate::service::ServiceError;
use crate::store::document::SearchDocRef;

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRequest {
    pub index_id: String,
    pub query: String,
    pub offset: usize,
    pub partition_n: usize,
    pub segments: Vec<SegmentMeta>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PartitionSearchHit {
    pub doc_ref: SearchDocRef,
    pub score: f32,
    pub partition_n: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PartitionQueryResponse {
    pub matches: Vec<PartitionSearchHit>,
}

pub async fn handle_event(
    index_loader: &dyn IndexLoader,
    event: LambdaEvent<QueryRequest>,
) -> Result<PartitionQueryResponse, Error> {
    let body = event.payload;
    let index_id = body.index_id;

    let index = index_loader.load_index(&index_id, Some(body.segments))?;

    let reader = index.reader().expect("Reader should load");

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

    let collector = TopDocs::with_limit(10).and_offset(body.offset);

    let top_docs: Vec<(Score, DocAddress)> = searcher
        .search(&query, &collector)
        .expect("search should succeed");

    let matches: Vec<_> = top_docs
        .into_iter()
        .map(|(score, address)| {
            let document = searcher.doc(address).expect("doc should exist");

            let named_doc = schema.to_named_doc(&document);

            let stored_ref = SearchDocRef::from(named_doc);

            PartitionSearchHit {
                doc_ref: stored_ref,
                score,
                partition_n: body.partition_n,
            }
        })
        .collect();

    if matches.len() == 0 {
        return Ok(PartitionQueryResponse { matches: vec![] });
    }

    Ok(PartitionQueryResponse { matches })
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
