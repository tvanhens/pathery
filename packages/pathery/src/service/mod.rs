use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;

use async_trait::async_trait;
use http::Response;
use lambda_http::{Body, RequestExt};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::util;

pub mod doc;
pub mod index;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("{0}")]
    InvalidRequest(String),

    #[error("Internal service error")]
    InternalError { id: String, source: anyhow::Error },

    #[error("Rate limit hit, back off and try request again.")]
    RateLimit,

    #[error("{0}")]
    NotFound(String),
}

impl ServiceError {
    pub fn invalid_request(message: &str) -> Self {
        ServiceError::InvalidRequest(message.into())
    }

    pub fn internal_error<E>(source: E) -> Self
    where E: Error + Send + Sync + 'static {
        let id = util::generate_id();
        error!(
            message = "InternalServiceError",
            id,
            error = format!("{source:#?}")
        );
        ServiceError::InternalError {
            id,
            source: anyhow::Error::new(source),
        }
    }

    pub fn not_found(message: &str) -> Self {
        ServiceError::NotFound(message.into())
    }

    pub fn rate_limit() -> Self {
        ServiceError::RateLimit
    }

    pub fn status(&self) -> u16 {
        use ServiceError::*;
        match self {
            InvalidRequest(_) => 400,
            InternalError { .. } => 500,
            RateLimit => 429,
            NotFound(_) => 404,
        }
    }

    pub fn message(self) -> String {
        use ServiceError::*;
        match self {
            InternalError { id, .. } => format!("Internal server error [id = {}]", id),
            InvalidRequest(message) => message,
            RateLimit => String::from("Too many requests"),
            NotFound(message) => message,
        }
    }
}

type ServiceResponse<R> = Result<R, ServiceError>;

pub struct ServiceRequest<B> {
    inner: lambda_http::Request,
    body: PhantomData<B>,
}

impl<B> ServiceRequest<B>
where B: for<'de> Deserialize<'de>
{
    /// Useful for testing
    pub fn create(body: B) -> ServiceRequest<B>
    where B: Serialize {
        let request = http::Request::builder();

        let body = lambda_http::Body::from(serde_json::to_string(&body).unwrap());

        let inner = request.body(body).unwrap();

        ServiceRequest {
            inner,
            body: PhantomData,
        }
    }

    /// Useful for testing
    pub fn with_path_param(mut self, name: &str, value: &str) -> Self {
        let updated = self
            .inner
            .with_path_parameters(HashMap::from([(String::from(name), String::from(value))]));

        self.inner = updated;

        self
    }

    pub fn body(&self) -> Result<B, ServiceError> {
        if let Body::Text(body) = self.inner.body() {
            Ok(serde_json::from_str(body).map_err(|err| {
                ServiceError::InvalidRequest(format!("Unable to parse body: {}", err.to_string()))
            })?)
        } else {
            Err(ServiceError::InvalidRequest(String::from(
                "Expected string for body",
            )))
        }
    }

    pub fn path_param(&self, name: &str) -> Result<String, ServiceError> {
        let path_params = self.inner.path_parameters();
        let value = path_params
            .first(name)
            .expect(&format!("missing path param: {}", name));

        Ok(String::from(value))
    }
}

fn map_error_response(
    error: ServiceError,
) -> Result<lambda_http::Response<lambda_http::Body>, lambda_http::Error> {
    let status = error.status();
    let message = error.message();

    let response = Response::builder()
        .header("Content-Type", "application/json")
        .status(status);

    let body = serde_json::to_string(&serde_json::json!({ "message": message }))?;

    Ok(response.body(Body::Text(body))?)
}

fn map_success_response<R>(
    response: R,
) -> Result<lambda_http::Response<lambda_http::Body>, lambda_http::Error>
where R: Serialize {
    let body = serde_json::to_string(&response)?;
    Ok(http::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(lambda_http::Body::Text(body))?)
}

#[async_trait]
pub trait ServiceHandler<B, R>: Sync
where
    B: for<'de> Deserialize<'de> + Send,
    R: Serialize,
{
    async fn handle_event(
        &self,
        event: lambda_http::Request,
    ) -> Result<lambda_http::Response<lambda_http::Body>, lambda_http::Error> {
        let request = ServiceRequest {
            inner: event,
            body: PhantomData,
        };

        self.handle_request(request)
            .await
            .map_or_else(map_error_response, map_success_response)
    }

    async fn handle_request(&self, request: ServiceRequest<B>) -> ServiceResponse<R>;
}

pub async fn start_service<B, R>(
    service: &dyn ServiceHandler<B, R>,
) -> Result<(), lambda_http::Error>
where
    B: for<'de> Deserialize<'de> + Send,
    R: Serialize,
{
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::WARN)
        .with_target(false)
        .without_time()
        .init();

    lambda_http::run(lambda_http::service_fn(|event| async {
        service.handle_event(event).await
    }))
    .await?;

    Ok(())
}
