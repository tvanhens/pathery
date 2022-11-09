mod directory;

use anyhow::Result;
use directory::DynamoDirectory;
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Schema, STORED, TEXT},
    DocAddress, Index, IndexSettings, Score,
};

fn main() -> Result<()> {
    let mut schema_builder = Schema::builder();
    let dir = DynamoDirectory::create("pathery-test", "abc-123")?;
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let schema = schema_builder.build();

    let index = Index::create(dir, schema.clone(), IndexSettings::default())?;

    // Here we use a buffer of 100MB that will be split
    // between indexing threads.
    let mut index_writer = index.writer(100_000_000)?;

    // Let's index one documents!
    index_writer.add_document(doc!(
        title => "The Old Man and the Sea",
        body => "He was an old man who fished alone in a skiff in \
                the Gulf Stream and he had gone eighty-four days \
                now without taking a fish."
    ))?;

    // We need to call .commit() explicitly to force the
    // index_writer to finish processing the documents in the queue,
    // flush the current index to the disk, and advertise
    // the existence of new documents.
    index_writer.commit()?;

    // # Searching

    let reader = index.reader()?;

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![title, body]);

    // QueryParser may fail if the query is not in the right
    // format. For user facing applications, this can be a problem.
    // A ticket has been opened regarding this problem.
    let query = query_parser.parse_query("sea whale")?;

    // Perform search.
    // `topdocs` contains the 10 most relevant doc ids, sorted by decreasing scores...
    let top_docs: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

    for (_score, doc_address) in top_docs {
        // Retrieve the actual content of documents given its `doc_address`.
        let retrieved_doc = searcher.doc(doc_address)?;
        println!("{}", schema.to_json(&retrieved_doc));
    }

    Ok(())
}
