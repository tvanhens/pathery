pub mod http;
pub mod sqs;

use std::panic;

pub use lambda_runtime;
pub use tracing;

pub use lambda_runtime::Error;

pub fn init_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    panic::set_hook(Box::new(|panic| {
        tracing::error!(message = panic.to_string());
    }));
}
