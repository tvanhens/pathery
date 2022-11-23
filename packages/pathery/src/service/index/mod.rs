mod batch_index;
mod post_index;
mod query_index;

pub use batch_index::batch_index;
pub use post_index::post_index;
pub use query_index::query_index;
use serde::{self, Deserialize, Serialize};

use crate::lambda::http::{self, HandlerResult};
use crate::schema::IndexDocError;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    index_id: String,
}

impl From<IndexDocError> for HandlerResult {
    fn from(err: IndexDocError) -> Self {
        match err {
            IndexDocError::EmptyDoc => {
                return Ok(http::err_response(400, "Request JSON object is empty"))
            }
            IndexDocError::NotJsonObject => {
                return Ok(http::err_response(400, "Expected JSON object"))
            }
            IndexDocError::DocParsingError(err) => {
                return Ok(http::err_response(400, &err.to_string()));
            }
        }
    }
}
