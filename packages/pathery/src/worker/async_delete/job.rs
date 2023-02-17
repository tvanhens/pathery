use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum AsyncDeleteJob {
    FSDelete(PathBuf),
}

impl AsyncDeleteJob {
    pub fn fs_delete(path: PathBuf) -> AsyncDeleteJob {
        AsyncDeleteJob::FSDelete(path)
    }
}
