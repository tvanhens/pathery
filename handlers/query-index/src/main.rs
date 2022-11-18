use pathery::index::{IndexLoader, IndexProvider};
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

pub fn search<L>(index_loader: &L, index_id: &str, query: &str) -> anyhow::Result<SearchResults>
where
    L: IndexLoader,
{
    let index = index_loader.load_index(index_id);

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

    let query = query_parser.parse_query(query)?;

    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

    let matches: Result<Vec<SearchHit>, _> = top_docs
        .into_iter()
        .map(|(score, address)| -> anyhow::Result<SearchHit> {
            let search_doc = searcher.doc(address)?;
            let mut doc_map = Map::new();
            let mut snippets: HashMap<String, String> = HashMap::new();

            for (field, entry) in index
                .schema()
                .fields()
                .filter(|(_, entry)| entry.is_indexed())
            {
                let field_name = entry.name();
                if let Some(value) = search_doc.get_first(field) {
                    let value = json::to_value(value)?;
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
    lambda::init_tracing();

    let index_loader = &IndexProvider::lambda();

    let handler = |event: http::Request| async move {
        let index_id = event.required_path_param("index_id");

        let payload = match event.payload::<QueryRequest>() {
            Ok(value) => value,
            Err(err) => return err.into(),
        };

        let results = search(index_loader, &index_id, &payload.query)?;

        http::success(&results)
    };

    http::run(http::service_fn(handler)).await
}

#[cfg(test)]
mod tests {
    use pathery::index::TantivyIndex;
    use pathery::tantivy::schema::Schema;
    use pathery::tantivy::{doc, schema, Index};
    use std::rc::Rc;

    use super::*;

    #[test]
    fn query_document_with_un_indexed_fields() {
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

        let results = search(&Rc::new(index), "test-index", "hello").unwrap();

        println!("{results:?}");

        assert_eq!(1, results.matches().len());
    }
}
