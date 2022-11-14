# API Docs

## General

The base url is the url of the API gateway and is emitted on installation via CDK.

**Example**

```
https://<api-id>.execute-api.us-east-1.amazonaws.com/prod
```

## Index Operations

### Index a Document

`POST /index/{index_name}`

Indexes a document so that the document is searchable.
A document can optionally provide an `__id` field to set the document id.
If no `__id` is provided one is generated and returned.
Indexing a document with an `__id` will upsert any previously indexed data with the provided `__id`.

**Example: Basic Indexing**

Request:

```bash
http https://<api-id>.execute-api.us-east-1.amazonaws.com/prod/index/book-index-1 title="Zen and the Art of Motorcycle Maintenance"
```

Response:

```json
{
  "__id": "b7c8aee4-9656-47a3-8217-df1b71056a83",
  "updated_at": "2022-11-14T21:17:58.824791120+00:00"
}
```

**Example: Providing an `\_\_id`**

```bash
http https://<api-id>.execute-api.us-east-1.amazonaws.com/prod/index/book-index-1 title="Zen and the Art of Motorcycle Maintenance" __id=zen
```

Response:

```json
{
  "__id": "zen",
  "updated_at": "2022-11-14T21:17:58.824791120+00:00"
}
```
