mod batch_index;
mod post_index;
mod query_index;
mod stats_index;

pub use batch_index::BatchIndexService;
pub use post_index::PostIndexService;
pub use query_index::QueryIndexService;
use serde::{self, Deserialize, Serialize};
pub use stats_index::stats_index;

use crate::lambda::http::{self, HandlerResult};
use crate::schema::IndexDocError;

#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    index_id: String,
}

impl From<IndexDocError> for HandlerResult {
    fn from(err: IndexDocError) -> Self {
        let message = match err {
            IndexDocError::DocParsingError(err) => err.to_string(),
            _ => err.to_string(),
        };
        Ok(http::err_response(400, &message))
    }
}
