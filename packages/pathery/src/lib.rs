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

    use crate::index::IndexLoader;
    use crate::schema::{SchemaLoader, SchemaProvider};
    use crate::store::document::test_util::TestDocumentStore;
    use crate::store::document::DocumentStore;
    use crate::worker::index_writer::client::test_utils::TestIndexWriterClient;

    pub struct TestContext {
        pub document_store: Box<dyn DocumentStore>,
        pub index_writer_client: TestIndexWriterClient,
        pub schema_loader: Box<dyn SchemaLoader>,
        pub index_loader: Box<dyn IndexLoader>,
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
                            "flags": ["TEXT", "STORED"]
                        },
                        {
                            "name": "author",
                            "kind": "text",
                            "flags": ["TEXT", "STORED"]
                        },
                        {
                            "name": "isbn",
                            "kind": "text",
                            "flags": ["STRING"]
                        },
                        {
                            "name": "date_added",
                            "kind": "date",
                            "flags": ["INDEXED", "STORED", "FAST"]
                        },
                        {
                            "name": "meta",
                            "kind": "text",
                            "flags": ["STORED"]
                        },
                        {
                            "name": "year",
                            "kind": "i64",
                            "flags": ["INDEXED", "STORED"]
                        }
                    ]
                }
            ]
        });

        let schema_provider = SchemaProvider::from_json(config);

        let index = Index::create_in_ram(schema_provider.load_schema("test"));

        TestContext {
            document_store: Box::new(TestDocumentStore::create()),
            index_writer_client: TestIndexWriterClient::create(),
            index_loader: Box::new(Arc::new(index)),
            schema_loader: Box::new(schema_provider),
        }
    }
}
