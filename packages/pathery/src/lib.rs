pub mod config;
pub mod directory;
pub mod indexer;

#[cfg(test)]
mod test {

    use std::vec;

    use anyhow::Result;
    use serde_json::json;
    use tantivy::{
        directory::RamDirectory,
        doc,
        schema::{Schema, TEXT},
        Index,
    };

    use crate::indexer::Indexer;

    #[test]
    fn write_sample_doc_to_indexer() -> Result<()> {
        let mut indexer = Indexer::create()?;

        indexer.index_doc(json!({
            "title": "The Old Man and the Sea",
            "body": "He was an old man who fished alone in a skiff in \
                    the Gulf Stream and he had gone eighty-four days \
                    now without taking a fish."
        }))?;

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
