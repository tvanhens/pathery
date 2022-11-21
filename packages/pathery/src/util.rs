use std::time::SystemTime;

use chrono::{DateTime, Utc};

pub fn generate_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()
}

pub fn timestamp() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}
