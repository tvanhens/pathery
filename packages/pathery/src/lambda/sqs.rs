use aws_lambda_events::event::sqs;
pub use lambda_runtime::Error;
use lambda_runtime::LambdaEvent;

pub type SqsEvent = LambdaEvent<sqs::SqsEvent>;
