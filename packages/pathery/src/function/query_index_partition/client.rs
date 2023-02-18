use aws_smithy_types::Blob;

use super::{PartitionQueryResponse, QueryRequest};
use crate::pagination::SegmentMeta;
use crate::util;

pub struct LambdaQueryIndexPartitionClient {
    function_name: String,

    client: aws_sdk_lambda::Client,
}

impl LambdaQueryIndexPartitionClient {
    pub async fn create() -> LambdaQueryIndexPartitionClient {
        let sdk_config = aws_config::load_from_env().await;
        let function_name = util::require_env("QUERY_INDEX_PARTITION_NAME");

        LambdaQueryIndexPartitionClient {
            function_name,
            client: aws_sdk_lambda::Client::new(&sdk_config),
        }
    }

    pub async fn query_partition(
        &self,
        index_id: String,
        query: String,
        offset: usize,
        partition_n: usize,
        segments: Vec<SegmentMeta>,
    ) -> PartitionQueryResponse {
        // TODO: Error handling and retries
        let request = self.client.invoke();
        let request = request.function_name(&self.function_name);
        let input = QueryRequest {
            index_id,
            query,
            offset,
            partition_n,
            segments,
        };
        let input = serde_json::to_vec(&input).expect("should serialize");
        let input = Blob::new(input);
        let request = request.payload(input);
        let response = request.send().await;
        let response = response.expect("should succeed");

        let payload = response.payload().expect("payload should exist");
        let payload = payload.to_owned().into_inner();
        let payload: PartitionQueryResponse =
            serde_json::from_slice(&payload).expect("payload should parse");
        payload
    }
}
