# Pathery

Pathery is an **open source serverless search service** built on AWS using Rust, CDK and [Tantivy][tantivy].

**Warning** This is currently a work in progress and not ready for real usage.

## Design

Pathery uses AWS DynamoDB for storage and AWS Lambda for code execution.
As a result, it is entirely usage-based and does not require any long-running server.

Rather than using the filesystem, pathery implements a [DynamoDB-backed][dynamodb-dir] Tantivy Directory - this limits any cold start overhead a VPC would introduce.
This makes it possible to spin up dozens of lambdas in parallel to execute distributed queries across all the segments in an index with minimal overhead.

## Todo

- [ ] Allow indexes to be configured at runtime
- [ ] Distributed queries
- [ ] Coordination for indexing
- [ ] Package and distrubute as a CDK Construct
- [ ] Cleanup indexing and query APIs

[tantivy]: https://github.com/quickwit-oss/tantivy
[dynamodb-dir]: packages/pathery/src/directory/mod.rs
