use pathery::index::IndexProvider;
use pathery::json::Map;
use pathery::lambda;
use pathery::lambda::{http, http::PatheryRequest};
use pathery::tantivy::collector::TopDocs;
use pathery::tantivy::query::QueryParser;
use pathery::tantivy::schema::Field;
use pathery::tantivy::{DocAddress, Score, SnippetGenerator};
use pathery::{anyhow, json, serde, tokio};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
#[serde(crate = "self::serde")]
struct QueryRequest {
    query: String,
}

#[derive(serde::Serialize, Debug)]
#[serde(crate = "self::serde")]
pub struct SearchHit {
    doc: json::Value,
    snippets: HashMap<String, String>,
    score: f32,
}

#[derive(serde::Serialize, Debug)]
#[serde(crate = "self::serde")]
pub struct SearchResults {
    matches: Vec<SearchHit>,
}

impl SearchResults {
    pub fn matches(&self) -> &Vec<SearchHit> {
        &self.matches
    }
}

pub fn search(index_id: &str, query: &str) -> anyhow::Result<SearchResults> {
    let index = IndexProvider::lambda_provider().load_index(index_id);

    let reader = index.reader().expect("Reader should load");

    let searcher = reader.searcher();

    let schema = index.schema();

    let query_parser = QueryParser::for_index(
        &index,
        schema
            .fields()
            .map(|(field, _)| field)
            .collect::<Vec<Field>>(),
    );

    let query = query_parser.parse_query(query)?;

    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

    let matches: Result<Vec<SearchHit>, _> = top_docs
        .into_iter()
        .map(|(score, address)| -> anyhow::Result<SearchHit> {
            let search_doc = searcher.doc(address)?;
            let mut doc_map = Map::new();
            let mut snippets: HashMap<String, String> = HashMap::new();

            for (field, entry) in index.schema().fields() {
                let field_name = entry.name();
                if let Some(value) = search_doc.get_first(field) {
                    let value = json::to_value(value)?;
                    doc_map.insert(field_name.to_string(), value);

                    let mut snippet_gen = SnippetGenerator::create(&searcher, &query, field)
                        .expect(&format!("Unable to create snippet for field: {field_name}"));
                    snippet_gen.set_max_num_chars(100);
                    let snippet_text = snippet_gen.snippet_from_doc(&search_doc).to_html();
                    if snippet_text.len() > 0 {
                        snippets.insert(field_name.into(), snippet_text);
                    }
                }
            }

            Ok(SearchHit {
                score,
                doc: doc_map.into(),
                snippets,
            })
        })
        .collect();

    Ok(SearchResults { matches: matches? })
}

#[tokio::main]
async fn main() -> Result<(), http::Error> {
    lambda::tracing_subscriber::fmt()
        .with_max_level(lambda::tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<QueryRequest>() {
            Ok(value) => value,
            Err(err) => return err.into(),
        };

        let results = search(&index_id, &payload.query)?;

        http::success(&results)
    };

    http::run(http::service_fn(handler)).await
}
