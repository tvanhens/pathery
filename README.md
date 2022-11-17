# Pathery :fire: Serverless Search :fire:

[![npm version](https://badge.fury.io/js/@pathery%2Fcdk.svg)](https://badge.fury.io/js/@pathery%2Fcdk)

Pathery is a **serverless search service** built on AWS using Rust, CDK and [Tantivy][tantivy].

**:bell: WARNING:** This is currently a work in progress and not ready for production usage.

## Features

- **🔥 Fast full-text search** - Pathery is built on Rust to limit its AWS Lambda cold start overhead.
- **🥰 Simple REST API** - Pathery exposes a [simple REST API][api-docs] to make search as easy as possible.
- **👍 Easy to install** - Pathery ships as a CDK Component making it easy to [get started][get-started].
- **💵 Usage based** - Pathery has no long running servers, only pay for what you use.
- **🔼 Built for AWS** - Pathery leans on AWS managed services to limit its maintenance burden and maximize its scalability.

[tantivy]: https://github.com/quickwit-oss/tantivy
[get-started]: ./doc/get-started.md
[api-docs]: ./doc/api.md

## Getting Started

Check out the [getting started guide][getting-started] to deploy Pathery into your AWS account using CDK.

[getting-started]: ./examples/getting-started/
