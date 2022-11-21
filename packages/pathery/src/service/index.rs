use std::collections::HashMap;

use json::Map;
use serde::{self, Deserialize, Serialize};
use serde_json::Value;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Field;
use tantivy::{DocAddress, Document, Score, SnippetGenerator};
use {serde_json as json, tracing};

use crate::index::IndexLoader;
use crate::lambda::http::{self, HandlerResult, ServiceRequest};
use crate::schema::{SchemaLoader, TantivySchema};
use crate::util;
use crate::worker::index_writer::client::IndexWriterClient;
use crate::worker::index_writer::op::IndexWriterOp;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    index_id: String,
}

#[derive(Serialize)]
pub struct PostIndexResponse {
    #[serde(rename = "__id")]
    pub doc_id: String,
    pub updated_at: String,
}

// Indexes a document supplied via a JSON object in the body.
#[tracing::instrument(skip(writer_client, schema_loader))]
pub async fn post_index(
    writer_client: &dyn IndexWriterClient,
    schema_loader: &dyn SchemaLoader,
    request: ServiceRequest<json::Value, PathParams>,
) -> HandlerResult {
    let (mut body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let schema = schema_loader.load_schema(&path_params.index_id);

    let doc_obj = if let Some(obj) = body.as_object_mut() {
        obj
    } else {
        return Ok(http::err_response(400, "Expected a JSON object"));
    };

    let doc_id = doc_obj
        .remove("__id")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(util::generate_id);

    let id_field = schema.id_field();

    let mut index_doc = Document::new();

    for (key, value) in doc_obj.iter() {
        match value {
            Value::String(v) => {
                if let Some(field) = schema.get_field(key) {
                    index_doc.add_text(field, v);
                }
            }
            _ => return Ok(http::err_response(400, "Unsupported JSON value in object")),
        };
    }

    if index_doc.is_empty() {
        // There are no fields that match the schema so the doc is empty
        return Ok(http::err_response(400, "Cannot index empty object"));
    }

    index_doc.add_text(id_field, &doc_id);

    writer_client
        .send_message(IndexWriterOp::index_single_doc(
            &path_params.index_id,
            index_doc,
        ))
        .await;

    http::success(&PostIndexResponse {
        doc_id,
        updated_at: util::timestamp(),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchHit {
    pub doc: json::Value,
    pub snippets: HashMap<String, String>,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug)]
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

    let index = index_loader.load_index(&path_params.index_id);

    let reader = index.reader().expect("Reader should load");

    let searcher = reader.searcher();

    let schema = index.schema();

    let query_parser = QueryParser::for_index(
        &index,
        schema
            .fields()
            .filter(|(_, config)| config.is_indexed())
            .map(|(field, _)| field)
            .collect::<Vec<Field>>(),
    );

    let query = query_parser.parse_query(&body.query)?;

    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

    let matches: Vec<_> = top_docs
        .into_iter()
        .map(|(score, address)| -> SearchHit {
            let search_doc = searcher.doc(address).expect("doc should exist");
            let mut doc_map = Map::new();
            let mut snippets: HashMap<String, String> = HashMap::new();

            for (field, entry) in index
                .schema()
                .fields()
                .filter(|(_, entry)| entry.is_indexed())
            {
                let field_name = entry.name();
                if let Some(value) = search_doc.get_first(field) {
                    let value =
                        json::to_value(value).expect("doc value should be JSON serializable");
                    doc_map.insert(field_name.to_string(), value);

                    let mut snippet_gen = SnippetGenerator::create(&searcher, &query, field)
                        .unwrap_or_else(|_| {
                            panic!("Unable to create snippet for field: {field_name}")
                        });
                    snippet_gen.set_max_num_chars(100);
                    let snippet_text = snippet_gen.snippet_from_doc(&search_doc).to_html();
                    if !snippet_text.is_empty() {
                        snippets.insert(field_name.into(), snippet_text);
                    }
                }
            }

            SearchHit {
                score,
                doc: doc_map.into(),
                snippets,
            }
        })
        .collect();

    http::success(&QueryResponse { matches })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use ::http::{Request, StatusCode};
    use aws_lambda_events::query_map::QueryMap;
    use lambda_http::{Body, RequestExt};
    use serde::Deserialize;
    use tantivy::schema::{self, Schema};
    use tantivy::{doc, Index};

    use super::*;
    use crate::index::TantivyIndex;
    use crate::lambda::http::{HandlerResponse, HttpRequest};
    use crate::schema::SchemaProvider;
    use crate::worker::index_writer::client::test_index_writer_client;

    fn setup() -> (impl IndexWriterClient, SchemaProvider) {
        let config = json::json!({
            "indexes": [
                {
                    "prefix": "test",
                    "fields": [
                        {
                            "name": "title",
                            "kind": "text",
                            "flags": ["TEXT"]
                        },
                        {
                            "name": "author",
                            "kind": "text",
                            "flags": ["TEXT"]
                        }
                    ]
                }
            ]
        });
        (
            test_index_writer_client(),
            SchemaProvider::from_json(config),
        )
    }

    fn request<B>(index_id: &str, body: B) -> ServiceRequest<B, PathParams>
    where B: Serialize {
        let request: HttpRequest = Request::builder()
            .header("Content-Type", "application/json")
            .body(json::to_string(&body).expect("should serialize").into())
            .expect("should build request");

        request
            .with_path_parameters::<QueryMap>(
                HashMap::from([(String::from("index_id"), String::from(index_id))]).into(),
            )
            .into()
    }

    fn parse_response<V>(response: HandlerResponse) -> (StatusCode, V)
    where V: for<'de> Deserialize<'de> {
        let code = response.status();
        let body: V = if let Body::Text(x) = response.body() {
            json::from_str(x).unwrap()
        } else {
            panic!("Invalid body")
        };
        (code, body)
    }

    #[tokio::test]
    async fn post_index_doc_with_no_id() {
        let (client, loader) = setup();

        let doc = json::json!({
            "title": "Zen and the Art of Motorcycle Maintenance",
            "author": "Robert Pirsig"
        });

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, _body) = parse_response::<json::Value>(response);

        assert_eq!(code, 200);
    }

    #[tokio::test]
    async fn post_index_non_object() {
        let (client, loader) = setup();

        let doc = json::json!([]);

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        assert_eq!(body, json::json!({"message": "Expected a JSON object"}));
    }

    #[tokio::test]
    async fn post_index_unsupported_value() {
        let (client, loader) = setup();

        let doc = json::json!({"foo": 1});

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        assert_eq!(
            body,
            json::json!({"message": "Unsupported JSON value in object"})
        );
    }

    #[tokio::test]
    async fn post_index_field_that_does_not_exist() {
        let (client, loader) = setup();

        let doc = json::json!({
            "foobar": "baz",
        });

        let request = request("test", doc);

        let response = post_index(&client, &loader, request).await.unwrap();

        let (code, body) = parse_response::<json::Value>(response);

        assert_eq!(code, 400);
        // Empty because the non-existent field does not explicitly trigger a failure - it just
        // doesn't get indexed.
        assert_eq!(body, json::json!({"message": "Cannot index empty object"}));
    }

    #[tokio::test]
    async fn query_document_with_un_indexed_fields() {
        let mut schema = Schema::builder();
        let title = schema.add_text_field("title", schema::STORED | schema::STRING);
        let author = schema.add_text_field("author", schema::STORED);
        let index = Index::create_in_ram(schema.build());
        let mut writer = index.default_writer();

        writer
            .add_document(doc!(
                title => "hello",
                author => "world",
            ))
            .unwrap();

        writer.commit().unwrap();

        let request = request(
            "test",
            QueryRequest {
                query: String::from("hello"),
            },
        );

        let response = query_index(&Arc::new(index), request).await.unwrap();

        let (status, body) = parse_response::<QueryResponse>(response);

        assert_eq!(200, status);
        assert_eq!(1, body.matches.len());
    }
}
