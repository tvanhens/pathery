use anyhow::Result;
use tantivy::{collector::TopDocs, query::QueryParser, schema::Field, DocAddress, Index, Score};

use crate::{directory::IndexerDirectory, index_loader::IndexLoader};

pub struct Searcher {
    index: Index,
}

impl Searcher {
    pub fn create(index_loader: &IndexLoader, index_id: &str) -> Result<Searcher> {
        let directory = IndexerDirectory::create(index_id);
        let index = Index::open_or_create(directory, index_loader.schema_for(index_id).unwrap())?;

        Ok(Searcher { index })
    }

    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

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

        let result: Result<Vec<String>, _> = top_docs
            .into_iter()
            .map(|(_score, address)| searcher.doc(address).map(|doc| schema.to_json(&doc)))
            .collect();

        Ok(result.unwrap())
    }
}
