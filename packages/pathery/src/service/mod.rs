use std::collections::HashMap;
use std::marker::PhantomData;

use async_trait::async_trait;
use lambda_http::{Body, RequestExt};
use serde::{Deserialize, Serialize};

pub mod doc;
pub mod index;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ServiceError {
    #[error("{0}")]
    InvalidRequest(String),

    #[error("{0}")]
    InternalError(String),

    #[error("Rate limit hit, back off and try request again.")]
    RateLimit,
}

impl ServiceError {
    pub fn invalid_request(message: &str) -> Self {
        ServiceError::InvalidRequest(message.into())
    }

    pub fn internal_error(message: &str) -> Self {
        ServiceError::InvalidRequest(message.into())
    }

    pub fn rate_limit() -> Self {
        ServiceError::RateLimit
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
    /// Used only for testing
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
        let value = path_params.first(name).ok_or_else(|| {
            ServiceError::InternalError(format!("Expected path parameter: {}", name))
        })?;

        Ok(String::from(value))
    }
}

fn map_error_response(
    error: ServiceError,
) -> Result<lambda_http::Response<lambda_http::Body>, lambda_http::Error> {
    match error {
        _ => todo!(),
    }
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
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    lambda_http::run(lambda_http::service_fn(|event| async {
        service.handle_event(event).await
    }))
    .await?;

    Ok(())
}
