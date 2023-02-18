# Pathery :fire: Serverless Search :fire:

[![npm version](https://badge.fury.io/js/@pathery%2Fcdk.svg)](https://badge.fury.io/js/@pathery%2Fcdk)

Pathery is a **serverless search service** built on AWS using Rust, CDK and [Tantivy][tantivy]. It uses AWS managed serverless offerings – DynamoDB, EFS, Lambda, and SQS – to the maximum extent possible.

**:bell: WARNING:** This is currently a work in progress and not ready for production usage.

## Features

- **🔥 Fast full-text search**. Built on Rust to limit AWS Lambda cold start overhead.
- **🥰 Simple REST API**. A [simple REST API][api-docs] to make search as easy as possible.
- **👍 Easy to install**. Ships as a CDK Component, making it easy to [get started][get-started].
- **💵 Usage based infra**. No long running servers, only pay for what you use.
- **🔼 Built for AWS**. Leans on AWS managed services to limit maintenance burden and maximize scalability.
  - Document store: [DynamoDB]([url](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Introduction.html))
  - Index store: [Elastic File System (EFS)]([url](https://docs.aws.amazon.com/efs/latest/ug/whatisefs.html))
  - Index writer & handler: [Lambda]([url](https://docs.aws.amazon.com/lambda/latest/dg/welcome.html))
  - Index queue: [SQS]([url](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/welcome.html)) 

## Getting Started

Check out the [getting started guide][get-started] to deploy Pathery into your AWS account using CDK.

[tantivy]: https://github.com/quickwit-oss/tantivy
[get-started]: ./examples/getting-started/
[api-docs]: ./doc/api.md
