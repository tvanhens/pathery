use serde::de::Visitor;
use serde::{Deserializer, Serializer};
use serde_json::{Map, Value};

pub fn serialize<S>(input: &Map<String, Value>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    let json_bytes = serde_json::to_vec(input).unwrap();
    let encoded_bytes = zstd::encode_all(json_bytes.as_slice(), 0).unwrap();
    serializer.serialize_bytes(&encoded_bytes)
}

struct CompressedJsonVisitor;

impl<'de> Visitor<'de> for CompressedJsonVisitor {
    type Value = Map<String, Value>;

    fn expecting(&self, _formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!()
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where E: serde::de::Error {
        let decoded_bytes = zstd::decode_all(v).unwrap();
        let deserialized = serde_json::from_slice(&decoded_bytes).unwrap();
        Ok(deserialized)
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Map<String, Value>, D::Error>
where D: Deserializer<'de> {
    deserializer.deserialize_bytes(CompressedJsonVisitor)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};
    use serde_dynamo::{self, AttributeValue};
    use serde_json::{json, Map, Value};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    struct MyType {
        #[serde(with = "super")]
        inner: Map<String, Value>,
    }

    #[test]
    fn test_round_trip() {
        let init = MyType {
            inner: json!({
                "hello": "world"
            })
            .as_object()
            .unwrap()
            .to_owned(),
        };

        let serialized: HashMap<String, AttributeValue> =
            serde_dynamo::to_item(init.clone()).unwrap();
        let deserialized: MyType = serde_dynamo::from_item(serialized).unwrap();

        assert_eq!(init, deserialized);
    }
}
