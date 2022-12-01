use std::marker::PhantomData;

use json::Map;
use lambda_http::{self as http, Body, RequestExt, Response};
use serde::{Deserialize, Serialize};
use serde_json as json;

pub type HttpRequest = http::Request;
pub type HandlerResponse = http::Response<http::Body>;
pub type HandlerResult = Result<HandlerResponse, http::Error>;

pub fn err_response(status: u16, message: &str) -> http::Response<http::Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json::json!({ "message": message }).to_string()))
        .expect("Failed to build response")
}

pub fn success<V>(value: &V) -> Result<http::Response<http::Body>, http::Error>
where V: Serialize {
    let value = json::to_string(value)?;
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(value))
        .expect("Failed to build response"))
}

#[derive(Debug)]
pub struct ServiceRequest<B, P> {
    request: http::Request,
    body: PhantomData<B>,
    path_params: PhantomData<P>,
}

impl<B, P> ServiceRequest<B, P>
where
    P: for<'de> Deserialize<'de>,
    B: for<'de> Deserialize<'de>,
{
    fn load_params(&self) -> Result<P, HandlerResponse> {
        let path_params = json::to_value(self.request.path_parameters())
            .expect("path_params should be serializable");

        let params_object = if let json::Value::Object(obj) = path_params {
            obj
        } else {
            panic!("path_params should be an object")
        };

        let flattened: Map<String, json::Value> = params_object
            .into_iter()
            .map(|(k, v)| match v {
                json::Value::Array(values) => (
                    k,
                    values
                        .into_iter().next()
                        .expect("param should have a value"),
                ),
                _ => panic!("keys should be array values"),
            })
            .collect();

        Ok(json::from_value(flattened.into()).unwrap())
    }

    fn load_body(&self) -> Result<B, HandlerResponse> {
        self.request
            .payload::<B>()
            .map_err(|err| panic!("{:?}", err))
            .and_then(|payload| payload.ok_or_else(|| err_response(400, "Missing body")))
    }

    pub fn into_parts(&self) -> Result<(B, P), HandlerResponse> {
        let body = self.load_body()?;
        let params = self.load_params()?;
        Ok((body, params))
    }
}

impl<B, P> From<http::Request> for ServiceRequest<B, P> {
    fn from(request: http::Request) -> Self {
        ServiceRequest {
            request,
            body: PhantomData::default(),
            path_params: PhantomData::default(),
        }
    }
}
