use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct PaginationToken {
    query: String,
    segment_ids: Vec<String>,
    partition_state: Vec<usize>,
}

impl PaginationToken {
    pub fn new<T>(query: T) -> PaginationToken
    where T: Into<String> {
        PaginationToken {
            query: query.into(),
            segment_ids: vec![],
            partition_state: vec![],
        }
    }

    pub fn set_segments(&mut self, segment_ids: Vec<String>) {
        self.segment_ids = segment_ids;
    }

    pub fn set_partition_state(&mut self, partition_state: Vec<usize>) {
        self.partition_state = partition_state;
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
    use super::PaginationToken;
    use crate::util;

    #[test]
    fn test_round_trip() {
        let mut token = PaginationToken::new("foobar");
        token.set_segments(vec![
            util::generate_id(),
            util::generate_id(),
            util::generate_id(),
            util::generate_id(),
        ]);
        token.set_partition_state(vec![2, 4]);

        let token_str = token.serialize();
        let parsed = PaginationToken::parse(token_str);

        assert_eq!(token, parsed);
    }
}
