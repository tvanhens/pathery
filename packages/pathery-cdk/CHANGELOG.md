# @pathery/cdk

## 0.2.4

### Patch Changes

- 371ab81: fix: run requests in parallel

## 0.2.3

### Patch Changes

- 3b2b99b: fix: stats endpoint missing env var

## 0.2.2

### Patch Changes

- 6e4ad31: Feature: automatically fan out queries as index grows
- 3142bbf: feat: add pagination via pagination token

## 0.2.1

### Patch Changes

- f4f3868: fix: run deletes async on delay queue

## 0.2.0

### Minor Changes

- b19295c: Compressed stored document representation.

## 0.1.1

### Patch Changes

- 61cd70b: Fix: documents were not serializing to writer queue correctly.
- 22598b6: Feature: Allow query handler memory size to be specified via CDK construct.
- ea2676c: Add json field type to schema config.
- 534908c: Fix: 404 error for missing index config
- 576d352: Feature: Partition queries using the optional with_partition body param.
- 534908c: Improvement: Use DynamoDB for original document storage.
- 534908c: Fix: allow empty body for delete doc request
- 653cd03: Feature: Add date field type.
- 61cd70b: Feature: add i64 as index field type

## 0.1.0

### Minor Changes

- 9ee82b6: Add API key authorization and generate default key.

### Patch Changes

- 83cb85c: Allow IndexWriter config to be specified.

## 0.0.9

### Patch Changes

- 03f647a: Add batch index endpoint

## 0.0.8

### Patch Changes

- 38a8116: Fix: incorrect dashboard naming

## 0.0.7

### Patch Changes

- 903af06: Add basic dashboard with errors and writer stats

## 0.0.6

### Patch Changes

- bf29fa3: Fixes https://github.com/tvanhens/pathery/issues/1

## 0.0.5

### Patch Changes

- 255c378: Add the STRING flag for text fields to enable exact-only matching.

## 0.0.4

### Patch Changes

- 8220a12: Improve package docs, keywords and description.

## 0.0.3

### Patch Changes

- c1e6d24: Add readme to package.

## 0.0.2

### Patch Changes

- 1e8060d: Move configuration into construct props.
