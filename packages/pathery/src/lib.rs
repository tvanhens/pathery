pub mod directory;
pub mod index;
pub mod lambda;
pub mod schema;
pub mod search_doc;
pub mod service;
pub mod store;
pub mod util;
pub mod worker;

pub(crate) use serde_json as json;

#[cfg(test)]
pub mod test_utils {
    use std::sync::Arc;

    pub use serde_json as json;
    pub use serde_json::json;
    use tantivy::Index;

    use crate::schema::{SchemaLoader, SchemaProvider};
    use crate::search_doc::SearchDoc;
    use crate::store::document::test_util::TestDocumentStore;
    use crate::store::document::DocumentStore;
    use crate::worker::index_writer::client::test_utils::TestIndexWriterClient;
    use crate::worker::index_writer::client::IndexWriterClient;
    use crate::worker::index_writer::job::Job;

    pub struct TestContext {
        pub document_store: Arc<TestDocumentStore>,
        pub index_writer_client: TestIndexWriterClient,
        pub schema_loader: Box<dyn SchemaLoader>,
        pub index_loader: Arc<Index>,
    }

    impl TestContext {
        pub async fn with_documents(self, docs: Vec<json::Value>) -> TestContext {
            let schema = self.schema_loader.load_schema("test");
            let documents: Vec<_> = docs
                .into_iter()
                .map(|value| SearchDoc::from_json(&schema, value).unwrap())
                .collect();
            let doc_refs = self.document_store.save_documents(documents).await.unwrap();
            let mut job = Job::create("test");
            for doc_ref in doc_refs {
                job.index_doc(doc_ref);
            }
            self.index_writer_client.submit_job(job).await.unwrap();
            self
        }
    }

    pub fn setup() -> TestContext {
        let config = json!({
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
                        },
                        {
                            "name": "isbn",
                            "kind": "text",
                            "flags": ["STRING"]
                        },
                        {
                            "name": "date_added",
                            "kind": "date",
                            "flags": ["INDEXED", "FAST"]
                        },
                        {
                            "name": "meta",
                            "kind": "text",
                            "flags": []
                        },
                        {
                            "name": "year",
                            "kind": "i64",
                            "flags": ["INDEXED"]
                        }
                    ]
                }
            ]
        });

        let schema_provider = SchemaProvider::from_json(config);

        let index = Arc::new(Index::create_in_ram(schema_provider.load_schema("test")));

        let document_store = Arc::new(TestDocumentStore::create());

        TestContext {
            index_writer_client: TestIndexWriterClient::create(&index, &document_store),
            document_store,
            index_loader: index,
            schema_loader: Box::new(schema_provider),
        }
    }
}
