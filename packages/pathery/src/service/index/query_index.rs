use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, FieldType};
use tantivy::{DocAddress, Score, SnippetGenerator, TantivyError};
use tracing::info;

use super::PathParams;
use crate::index::IndexLoader;
use crate::json;
use crate::lambda::http::{self, HandlerResult, ServiceRequest};

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

pub async fn query_index(
    index_loader: &dyn IndexLoader,
    request: ServiceRequest<QueryRequest, PathParams>,
) -> HandlerResult {
    let (body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let index = index_loader.load_index(
        &path_params.index_id,
        body.with_partition
            .map(|x| (x.partition_n, x.total_partitions)),
    );

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

    let query = query_parser.parse_query(&body.query)?;

    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

    let matches: Vec<_> = top_docs
        .into_iter()
        .map(|(score, address)| -> SearchHit {
            let document = searcher.doc(address).expect("doc should exist");

            let named_doc = schema.to_named_doc(&document);

            let snippets: HashMap<String, String> = document
                .field_values()
                .iter()
                .filter_map(|field_value| {
                    // Only text fields are supported for snippets
                    let text = field_value.value().as_text()?;

                    let generator =
                        match SnippetGenerator::create(&searcher, &query, field_value.field()) {
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

    let query_response = &QueryResponse { matches };

    http::success(query_response)
}

#[cfg(test)]
mod tests {
    use tantivy::IndexWriter;

    use super::*;
    use crate::index::IndexExt;
    use crate::schema::SchemaExt;
    use crate::service::index::PathParams;
    use crate::service::test_utils::*;

    trait WriterExt {
        fn add_json_doc(&self, json_doc: json::Value);
    }

    impl WriterExt for IndexWriter {
        fn add_json_doc(&self, json_doc: json::Value) {
            let schema = self.index().schema();

            let (_id, document) = schema.to_document(json_doc).unwrap();

            self.add_document(document).unwrap();
        }
    }

    #[tokio::test]
    async fn query_default_response() {
        let TestContext { index_loader, .. } = setup();

        let index = index_loader.load_index("test", None);

        let mut writer = index.default_writer();

        writer.add_json_doc(json::json!({
            "__id": "foobar",
            "title": "hello",
            "author": "world"
        }));

        writer.commit().unwrap();

        let request = request(
            QueryRequest {
                query: "hello".into(),
                with_partition: None,
            },
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = query_index(&index, request).await.unwrap();

        let (status, body) = parse_response::<QueryResponse>(response);

        assert_eq!(200, status);
        assert_eq!(
            body,
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
            }
        );
    }

    #[tokio::test]
    async fn query_document_with_un_indexed_fields() {
        let TestContext { index_loader, .. } = setup();

        let index = index_loader.load_index("test", None);

        let mut writer = index.default_writer();

        writer.add_json_doc(json::json!({
            "__id": "foobar",
            "title": "hello",
            "meta": "world"
        }));

        writer.commit().unwrap();

        let request = request(
            QueryRequest {
                query: "hello".into(),
                with_partition: None,
            },
            PathParams {
                index_id: "test".into(),
            },
        );

        let response = query_index(&index, request).await.unwrap();

        let (status, body) = parse_response::<QueryResponse>(response);

        assert_eq!(200, status);
        assert_eq!(1, body.matches.len());
    }
}
