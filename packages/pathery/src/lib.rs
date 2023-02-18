pub mod directory;
pub mod function;
pub mod index;
pub mod lambda;
pub mod pagination;
pub mod schema;
pub mod search_doc;
pub mod serialize;
pub mod service;
pub mod store;
pub mod util;
pub mod worker;

pub(crate) use serde_json as json;

#[cfg(test)]
pub mod test_utils {
    pub use serde_json as json;
    pub use serde_json::json;

    use crate::index::test_util::TestIndexLoader;
    use crate::schema::{SchemaLoader, SchemaProvider};
    use crate::search_doc::SearchDoc;
    use crate::store::document::test_util::TestDocumentStore;
    use crate::store::document::DocumentStore;
    use crate::worker::index_writer::client::test_utils::TestIndexWriterClient;
    use crate::worker::index_writer::client::IndexWriterClient;
    use crate::worker::index_writer::job::Job;

    pub struct TestContext {
        schema_loader: SchemaProvider,

        document_store: TestDocumentStore,

        writer_client: TestIndexWriterClient,

        index_loader: TestIndexLoader,
    }

    impl TestContext {
        pub async fn with_documents(self, index_id: &str, docs: Vec<json::Value>) -> TestContext {
            let schema = self.schema_loader.load_schema(index_id).unwrap();
            let documents: Vec<_> = docs
                .into_iter()
                .map(|value| SearchDoc::from_json(&schema, value).unwrap())
                .collect();
            let doc_refs = self.document_store.save_documents(documents).await.unwrap();
            let mut job = Job::create(index_id);
            for doc_ref in doc_refs {
                job.index_doc(doc_ref);
            }
            self.writer_client().submit_job(job).await.unwrap();
            self
        }

        pub fn schema_loader(&self) -> &SchemaProvider {
            &self.schema_loader
        }

        pub fn document_store(&self) -> &TestDocumentStore {
            &self.document_store
        }

        pub fn writer_client(&self) -> &TestIndexWriterClient {
            &self.writer_client
        }

        pub fn index_loader(&self) -> &TestIndexLoader {
            &self.index_loader
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
                        },
                        {
                            "name": "props",
                            "kind": "json",
                            "flags": ["TEXT"]
                        }
                    ]
                }
            ]
        });

        let schema_loader = SchemaProvider::from_json(config);

        let index_loader = TestIndexLoader::create(schema_loader.clone());

        let document_store = TestDocumentStore::create();

        TestContext {
            schema_loader,
            writer_client: TestIndexWriterClient::create(
                index_loader.clone(),
                document_store.clone(),
            ),
            document_store,
            index_loader,
        }
    }
}
