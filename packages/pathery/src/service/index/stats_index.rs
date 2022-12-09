use std::fs;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::index::{IndexLoader, IndexProvider};
use crate::service::{ServiceHandler, ServiceRequest, ServiceResponse};

#[derive(Serialize, Deserialize)]
pub struct SegmentStats {
    id: String,
    num_docs: u32,
    num_deleted: u32,
    index_size: f64,
}

#[derive(Serialize, Deserialize)]
pub struct IndexStatsResponse {
    segments: Vec<SegmentStats>,
}

pub struct StatsIndexService {
    index_loader: Box<dyn IndexLoader>,
}

#[async_trait]
impl ServiceHandler<json::Value, IndexStatsResponse> for StatsIndexService {
    async fn handle_request(
        &self,
        request: ServiceRequest<json::Value>,
    ) -> ServiceResponse<IndexStatsResponse> {
        let index_id = request.path_param("index_id")?;

        let index = self.index_loader.load_index(&index_id, None);

        let metas = index.load_metas().unwrap();

        let segment_files = fs::read_dir(format!("/mnt/pathery-data/{index_id}"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();

        let segments = metas
            .segments
            .iter()
            .map(|s| {
                let segment_id = s.id().uuid_string();

                let index_size_bytes: u64 = segment_files
                    .iter()
                    .filter_map(|entry| {
                        let filename = entry.file_name();
                        let filename = filename.to_str()?;

                        filename
                            .starts_with(&segment_id)
                            .then(|| entry.metadata())
                            .and_then(Result::ok)
                            .map(|m| m.len())
                    })
                    .sum();

                let index_size_mb: f64 = index_size_bytes as f64 / 1_000_000f64;

                SegmentStats {
                    id: s.id().uuid_string(),
                    num_docs: s.num_docs(),
                    num_deleted: s.num_deleted_docs(),
                    index_size: index_size_mb,
                }
            })
            .collect();

        Ok(IndexStatsResponse { segments })
    }
}

impl StatsIndexService {
    pub async fn create() -> Self {
        let index_loader = IndexProvider::lambda();

        StatsIndexService {
            index_loader: Box::new(index_loader),
        }
    }
}
