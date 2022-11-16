pub use lambda_runtime::Error;

use aws_lambda_events::event::sqs;
use lambda_runtime::LambdaEvent;

pub type SqsEvent = LambdaEvent<sqs::SqsEvent>;
