pub use lambda_http::{
    self as http, run, service_fn, Body, Error, IntoResponse, Request, RequestExt, Response,
};

use serde::{Deserialize, Serialize};
use serde_json as json;

fn response(status: u16, message: &str) -> http::Response<http::Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json::json!({ "message": message }).to_string()))
        .expect("Failed to build missing body response")
}

pub fn success<V>(value: &V) -> Result<http::Response<http::Body>, http::Error>
where
    V: Serialize,
{
    let json_str = json::to_string(value)?;
    Ok(response(200, &json_str))
}

pub enum PatheryHttpError {
    MissingBody,
}

impl From<PatheryHttpError> for Result<http::Response<http::Body>, http::Error> {
    fn from(err: PatheryHttpError) -> Self {
        match err {
            PatheryHttpError::MissingBody => Ok(response(400, "Missing body")),
        }
    }
}

pub trait PatheryRequest {
    fn required_path_param(&self, name: &str) -> String;
    fn payload<T>(&self) -> Result<T, PatheryHttpError>
    where
        T: for<'de> Deserialize<'de>;
}

impl PatheryRequest for http::Request {
    fn required_path_param(&self, name: &str) -> String {
        let params = self.path_parameters();
        let found = params
            .first(name)
            .expect(&format!("Expected path param not found: [{name}]"));
        found.to_string()
    }

    fn payload<T>(&self) -> Result<T, PatheryHttpError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let payload = RequestExt::payload::<T>(self);

        match payload {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(PatheryHttpError::MissingBody),
            Err(_) => todo!(),
        }
    }
}
