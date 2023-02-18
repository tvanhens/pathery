use std::collections::HashMap;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SegmentMeta {
    segment_id: String,

    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct PaginationToken {
    query: String,
    segments: Vec<SegmentMeta>,
    partition_state: Vec<usize>,
}

impl PaginationToken {
    pub fn new<T>(query: T, total_partitions: usize) -> PaginationToken
    where T: Into<String> {
        let mut partition_state: Vec<usize> = vec![];
        partition_state.resize(total_partitions, 0);
        PaginationToken {
            query: query.into(),
            segments: vec![],
            partition_state,
        }
    }

    pub fn import_segments_json(&mut self, segments_json: Value) {
        let segments: Vec<SegmentMeta> = serde_json::from_value(segments_json).unwrap();
        self.segments = segments;
    }

    pub fn segments_for_partition(&self, n: usize) -> Vec<SegmentMeta> {
        self.segments
            .iter()
            .enumerate()
            .filter(|(idx, _)| (idx + n) % self.partition_state.len() == 0)
            .map(|(_, x)| x.clone())
            .collect()
    }

    pub fn inc_offset(&mut self, partition_n: usize) {
        let value = self.partition_state.get_mut(partition_n).unwrap();
        *value = *value + 1;
    }

    pub fn get_offset(&self, partition_n: usize) -> usize {
        *self.partition_state.get(partition_n).unwrap()
    }

    pub fn get_query(&self) -> String {
        self.query.to_string()
    }

    pub fn serialize(&self) -> String {
        let json = serde_json::to_vec(self).expect("should serialize to json");
        let compressed = zstd::encode_all(json.as_slice(), 20).expect("should encode");
        base64::engine::general_purpose::STANDARD.encode(compressed)
    }

    pub fn parse<T>(from: T) -> PaginationToken
    where T: Into<String> {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(from.into())
            .unwrap();
        let decompressed = zstd::decode_all(decoded.as_slice()).unwrap();
        serde_json::from_slice(&decompressed).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::PaginationToken;

    #[test]
    fn test_round_trip() {
        let mut token = PaginationToken::new("foobar", 2);
        token.import_segments_json(json!([
            {
                "segment_id": "abc123",
                "foo": "bar"
            }
        ]));

        let token_str = token.serialize();
        let parsed = PaginationToken::parse(token_str);

        println!("{:?}", parsed);

        assert_eq!(token, parsed);
    }
}
