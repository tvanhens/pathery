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

pub fn require_env(var_name: &str) -> String {
    std::env::var(var_name).expect(&format!("{var_name:?} should be set"))
}
