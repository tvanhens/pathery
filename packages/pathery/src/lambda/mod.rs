pub mod sqs;

pub use lambda_runtime::Error;
pub use {lambda_runtime, tracing};

pub fn init_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .without_time()
        .init();
}
