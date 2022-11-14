use crate::{directory::IndexerDirectory, index_loader::IndexLoader};
use anyhow::Result;
use aws_sdk_dynamodb::Client as DDBClient;
use serde::Serialize;
use serde_json::Map;
use std::sync::Arc;
use tantivy::{
    collector::TopDocs, query::QueryParser, schema::Field, DocAddress, Index, IndexReader, Score,
};

#[derive(Serialize)]
pub struct SearchHit {
    doc: serde_json::Value,
    score: f32,
}

#[derive(Serialize)]
pub struct SearchResults {
    matches: Vec<SearchHit>,
}

impl SearchResults {
    pub fn matches(&self) -> &Vec<SearchHit> {
        &self.matches
    }
}

pub struct Searcher {
    index: Index,
    reader: IndexReader,
}

impl Searcher {
    pub fn create(
        client: &Arc<DDBClient>,
        index_loader: &IndexLoader,
        index_id: &str,
    ) -> Result<Searcher> {
        tokio::task::block_in_place(|| {
            let directory = IndexerDirectory::create(client, index_id);
            let index =
                Index::open_or_create(directory, index_loader.schema_for(index_id).unwrap())?;

            let reader = index.reader()?;

            Ok(Searcher { index, reader })
        })
    }

    pub fn search(&self, query: &str) -> Result<SearchResults> {
        let searcher = self.reader.searcher();

        let schema = self.index.schema();

        let query_parser = QueryParser::for_index(
            &self.index,
            schema
                .fields()
                .map(|(field, _)| field)
                .collect::<Vec<Field>>(),
        );

        let query = query_parser.parse_query(query)?;

        let top_docs: Vec<(Score, DocAddress)> =
            searcher.search(&query, &TopDocs::with_limit(10))?;

        let matches: Result<Vec<SearchHit>, _> = top_docs
            .into_iter()
            .map(|(score, address)| -> Result<SearchHit> {
                let search_doc = searcher.doc(address)?;
                let mut doc_map = Map::new();

                for (field, entry) in self.index.schema().fields() {
                    let field_name = entry.name();
                    if let Some(value) = search_doc.get_first(field) {
                        let value = serde_json::to_value(value)?;
                        doc_map.insert(field_name.to_string(), value);
                    }
                }

                Ok(SearchHit {
                    score,
                    doc: doc_map.into(),
                })
            })
            .collect();

        Ok(SearchResults { matches: matches? })
    }
}
