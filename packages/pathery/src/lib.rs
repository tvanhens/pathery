pub mod indexer;

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

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
}
