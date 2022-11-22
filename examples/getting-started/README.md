# Getting Started

This guide will walk you through:

1. Project Setup
1. Pathery Deployment
1. Writing documents to an index
1. Querying an index

## Project Setup

Pathery ships as a CDK Construct and requires TypeScript and AWS CDK to be installed.
The minimum set of dependencies is shown below:

**package.json**

```json
{
  "name": "getting-started",
  "version": "0.0.0",
  "description": "",
  "main": "index.js",
  "scripts": {
    "deploy": "cdk deploy"
  },
  "keywords": [],
  "author": "",
  "license": "ISC",
  "dependencies": {
    "@pathery/cdk": "^0.0.4",
    "@swc/core": "^1.3.14",
    "@types/node": "^18.11.9",
    "aws-cdk": "^2.50.0",
    "aws-cdk-lib": "^2.50.0",
    "constructs": "^10.1.155",
    "ts-node": "^10.9.1",
    "typescript": "^4.8.4"
  }
}
```

Running `npm install` will install the required dependencies.
Next, you can configure your first index pattern in `src/app.ts`.
Index patterns define the field configuration for indexes that start with the given prefix.

In the example below, any index that starts with the name `book-index-v1-` will have the fields `author` and `title` indexed.
You can read more about index configuration in the [index configuration guide][index-config].

**src/app.ts**

```typescript
import { App } from "aws-cdk-lib";
import { PatheryStack } from "@pathery/cdk";

const app = new App();

new PatheryStack(app, "pathery-dev", {
  config: {
    indexes: [
      {
        // Indexes starting with this prefix will use this config
        prefix: "book-index-v1-",
        fields: [
          {
            // Index the field title
            name: "title",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
          {
            // Index the field author
            name: "author",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
        ],
      },
    ],
  },
});
```

Lastly, CDK needs to know where our CDK app is declared so we include a `cdk.json`:

**cdk.json**

```json
{
  "app": "ts-node --swc src/app.ts"
}
```

This is the minimum amount of setup required. Now we can deploy our Pathery search service.

## Deployment

To deploy the project run `npm run deploy`.

If everything worked, you should see an output that looks like the one below:

```bash
  ✅  pathery-dev

 ✨  Deployment time: 55.94s

 Outputs:
 arn:aws:cloudformation:us-east-1:117773642559:stack/pathery-dev/f1c49c40-60b3-11ed-b19f-0e7f8a5bfcb7
 pathery-dev.ApiKeyOutput = <omitted>
 pathery-dev.PatheryApiEndpointB5297505 = https://<omitted>.execute-api.us-east-1.amazonaws.com/prod/
 Stack ARN:

 ✨  Total time: 58.13s
```

Note the output called `pathery-dev.PatheryApiEndpointB5297505`, this is the URL to your search API.
Lets save it to your shell environment for the next step by running:

```bash
export PATHERY_ENDPOINT=<url from PatheryApiEndpoint output above>
```

This endpoint is authenticated using an API key that gets automatically generated.
Copy the id on the right hand side of the output `<stack-name>.ApiKeyOutput = <omitted>` and paste it into the line below for `<api-key-id>`:

```bash
export PATHERY_KEY="$(aws apigateway get-api-key --include-value --api-key <api-key-id> --query value --output text)"
```

[index-config]: ../../doc/index-config.md

## Indexing a Document

To index an example document run:

```bash
curl -X POST ${PATHERY_ENDPOINT}index/book-index-v1-test \
     -H 'Content-Type: application/json' \
     -H "x-api-key: ${PATHERY_KEY}" \
     -d '{"title": "Zen and the Art of Motorcycle Maintenance", "author": "Robert Pirsig"}'
```

> **❕ Note**
>
> Our index is name is `book-index-v1-test`.
> The prefix of `book-index-v1-` is required to match the prefix in our configuration.
>
> **If you try to post to an index which does not match a configuration prefix, the request will fail.**

If indexing is successful you should see:

```json
{
  "__id": "7a309cda-1314-4e0a-a97d-02ce2c5e24c7",
  "updated_at": "2022-11-17T17:49:28.835542383+00:00"
}
```

Now we're ready to query our index.

## Querying an Index

To query our index we can use a request like the one below:

```bash
curl -X POST ${PATHERY_ENDPOINT}index/book-index-v1-test/query \
     -H 'Content-Type: application/json' \
     -H "x-api-key: ${PATHERY_KEY}" \
     -d '{"query": "zen art pirsig"}'
```

You should see a response like the one below, note the matching search terms are highlighted in the `snippets` of the response:

```json
{
  "matches": [
    {
      "doc": {
        "__id": "7a309cda-1314-4e0a-a97d-02ce2c5e24c7",
        "author": "Robert Pirsig",
        "title": "Zen and the Art of Motorcycle Maintenance"
      },
      "snippets": {
        "title": "<b>Zen</b> and the <b>Art</b> of Motorcycle Maintenance",
        "author": "Robert <b>Pirsig</b>"
      },
      "score": 0.86304635
    }
  ]
}
```
