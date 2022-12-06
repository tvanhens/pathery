use std::fs;

use serde::{Deserialize, Serialize};
use serde_json as json;

use super::PathParams;
use crate::index::IndexLoader;
use crate::lambda::http::{success, HandlerResult, ServiceRequest};

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

pub async fn stats_index(
    index_loader: &dyn IndexLoader,
    request: ServiceRequest<Option<json::Value>, PathParams>,
) -> HandlerResult {
    let (_body, path_params) = match request.into_parts() {
        Ok(parts) => parts,
        Err(response) => return Ok(response),
    };

    let index_id = path_params.index_id;

    let index = index_loader.load_index(&index_id, None);

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

    success(&IndexStatsResponse { segments })
}
