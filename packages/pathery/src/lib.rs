pub mod config;
pub mod directory;
pub mod index_loader;
pub mod indexer;
pub mod lambda;
pub mod searcher;

#[cfg(test)]
mod test {

    use std::vec;

    use crate::{index_loader::IndexLoader, indexer::Indexer, lambda, searcher::Searcher};
    use anyhow::Result;
    use serde_json::json;
    use tantivy::{
        directory::RamDirectory,
        doc,
        schema::{Schema, TEXT},
        Index,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn write_sample_doc_to_indexer_and_query() -> Result<()> {
        let loader = IndexLoader::create("../../app/config/pathery-config")?;
        let index_id = format!("book-index-{}", uuid::Uuid::new_v4().to_string());
        let client = lambda::ddb_client().await;

        let mut indexer = Indexer::create(&client, &loader, &index_id)?;
        let mut doc = json!({
            "title": "The Old Man and the Sea",
            "body": "He was an old man who fished alone in a skiff in \
                    the Gulf Stream and he had gone eighty-four days \
                    now without taking a fish."
        });

        indexer.index_doc(&mut doc)?;

        let searcher = Searcher::create(&client, &loader, &index_id)?;

        let results = searcher.search("Gulf")?;

        assert_eq!(1, results.len());

        Ok(())
    }

    #[test]
    fn can_split_index() -> Result<()> {
        let index_1 = {
            let mut schema_builder = Schema::builder();
            let text_field = schema_builder.add_text_field("text", TEXT);
            let index = Index::create_in_ram(schema_builder.build());
            let mut index_writer = index.writer(3_000_000)?;

            index_writer.add_document(doc!(text_field=>"texto1"))?;
            index_writer.add_document(doc!(text_field=>"texto2"))?;
            index_writer.commit()?;

            index_writer.add_document(doc!(text_field=>"texto3"))?;
            index_writer.add_document(doc!(text_field=>"texto4"))?;
            index_writer.commit()?;

            index
        };

        let index_2 = tantivy::merge_filtered_segments(
            &[index_1.searchable_segments()?[0].to_owned()],
            index_1.settings().to_owned(),
            vec![None],
            RamDirectory::default(),
        )?;

        let reader = index_2.reader()?;
        reader.reload()?;

        assert_eq!(2, reader.searcher().num_docs());

        Ok(())
    }
}
