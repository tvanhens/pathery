pub mod client;
pub mod job;

use std::fs;
use std::path::PathBuf;

use serde_json as json;

use crate::lambda::{self, sqs};

pub fn fs_delete(path: PathBuf) {
    fs::remove_file(path).expect("should be able to delete file");
}

pub async fn handle_event(event: sqs::SqsEvent) -> Result<(), lambda::Error> {
    let records = event.payload.records;

    let jobs = records
        .iter()
        .map(|message| message.body.as_ref().expect("Body should be present"))
        .map(|body| {
            let msg = json::from_str::<job::AsyncDeleteJob>(body.as_str())
                .expect("Message should be deserializable");
            msg
        })
        .collect::<Vec<_>>();

    for ele in jobs {
        print!("{:?}", ele);
        match ele {
            job::AsyncDeleteJob::FSDelete(path) => fs_delete(path),
        }
    }

    Ok(())
}
