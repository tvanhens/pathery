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

#### Parameters

- `__id` - (optional) the document id to use for the document

#### Examples

**Basic Indexing**

Request:

```bash
http https://<api-id>.execute-api.us-east-1.amazonaws.com/prod/index/book-index-1 \
     author="Robert M. Pirsig" \
     title="Zen and the Art of Motorcycle Maintenance"
```

Response:

```json
{
  "__id": "b7c8aee4-9656-47a3-8217-df1b71056a83",
  "updated_at": "2022-11-14T21:17:58.824791120+00:00"
}
```

**Providing an `\_\_id`**

Request:

```bash
http https://<api-id>.execute-api.us-east-1.amazonaws.com/prod/index/book-index-1 \
     author="Robert M. Pirsig" \
     title="Zen and the Art of Motorcycle Maintenance" \
     __id=zen
```

Response:

```json
{
  "__id": "zen",
  "updated_at": "2022-11-14T21:17:58.824791120+00:00"
}
```

### Query a Document

`POST /index/{index_name}/query`

Query an index with a provided search string.

#### Parameters

- `query` - a query string to search against the index

#### Examples

**Simple Full Text Search**

Request:

```bash
http https://<api-id>.execute-api.us-east-1.amazonaws.com/prod/index/book-index-1/query \
     query="Zen"
```

Response:

```json
{
  "matches": [
    {
      "doc": {
        "__id": "44764c78-1c94-4a03-9102-e2d9e3d9967b",
        "title": "Zen and the Art of Motorcycle Maintenance"
      },
      "score": 0.28768211603164673
    }
  ]
}
```
