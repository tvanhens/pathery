pub mod doc;
pub mod index;

#[cfg(test)]
mod test_utils {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::vec;

    use ::http::{Request, StatusCode};
    use aws_lambda_events::query_map::QueryMap;
    use lambda_http::{Body, RequestExt};
    use serde::{Deserialize, Serialize};
    pub use tantivy::doc;
    use tantivy::Index;

    pub(crate) use crate::json;
    use crate::lambda::http::{HandlerResponse, HttpRequest, ServiceRequest};
    use crate::schema::{SchemaLoader, SchemaProvider};
    use crate::worker::index_writer::client::IndexWriterClient;

    fn test_index_writer_client() -> Box<dyn IndexWriterClient> {
        todo!()
    }

    pub fn setup() -> (Box<dyn IndexWriterClient>, SchemaProvider, Arc<Index>) {
        let config = json::json!({
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
                        }
                    ]
                }
            ]
        });

        let schema_provider = SchemaProvider::from_json(config);

        let index = Index::create_in_ram(schema_provider.load_schema("test"));

        (test_index_writer_client(), schema_provider, Arc::new(index))
    }

    pub fn request<B, P>(body: B, params: P) -> ServiceRequest<B, P>
    where
        B: Serialize,
        P: Serialize,
    {
        let request: HttpRequest = Request::builder()
            .header("Content-Type", "application/json")
            .body(json::to_string(&body).expect("should serialize").into())
            .expect("should build request");

        let params_value = json::to_value(params).expect("params should serialize to value");

        let params_map: HashMap<String, String> =
            json::from_value(params_value).expect("params value should deserialize");

        request
            .with_path_parameters::<QueryMap>(params_map.into())
            .into()
    }

    pub fn parse_response<V>(response: HandlerResponse) -> (StatusCode, V)
    where V: for<'de> Deserialize<'de> {
        let code = response.status();
        let body: V = if let Body::Text(x) = response.body() {
            json::from_str(x).unwrap()
        } else {
            panic!("Invalid body")
        };
        (code, body)
    }
}
