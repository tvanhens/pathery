pub mod doc;
pub mod index;

#[cfg(test)]
mod test_utils {
    use std::collections::HashMap;

    use ::http::{Request, StatusCode};
    use aws_lambda_events::query_map::QueryMap;
    use lambda_http::{Body, RequestExt};
    use serde::{Deserialize, Serialize};
    pub use tantivy::doc;

    use crate::lambda::http::{HandlerResponse, HttpRequest, ServiceRequest};
    pub use crate::test_utils::*;

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
