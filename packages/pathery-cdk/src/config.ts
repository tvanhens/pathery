export interface FieldConfig<K, Flags> {
  /**
   * The name of the field to index.
   *
   * This must match the object key name of objects being indexed.
   */
  name: string;

  /**
   * The kind of field.
   *
   * Kind descriptions:
   *
   * `text` - Indexes field values as `string`.
   *
   * `date` - Indexes field values as ints but serialized as ISO 80601 strings in transit.
   */
  kind: K;

  /**
   * Flags to add additional indexing capabilities.
   *
   * Flag descriptions:
   *
   *
   * `TEXT`    - (only for `text`) Marks this field for full-text indexing.
   *
   * `STRING`  - (only for `text`) Marks this field for exact-string indexing.
   *
   * `INDEXED` - (only for `date`) Marks this field for ordered search indexing.
   */
  flags: Flags[];
}

export type TextFieldConfig = FieldConfig<"text", "STRING" | "TEXT" | "FAST">;

export type DateFieldConfig = FieldConfig<"date", "INDEXED" | "FAST">;

export type IntegerFieldConfig = FieldConfig<"i64", "INDEXED" | "FAST">;

export type JsonFieldConfig = FieldConfig<"json", "TEXT">;

export type IndexFieldConfig =
  | TextFieldConfig
  | DateFieldConfig
  | IntegerFieldConfig
  | JsonFieldConfig;

export interface IndexConfig {
  /**
   * Prefix matcher for index name.
   *
   * Indexes that start with `prefix` will use the fields schema and configuration specified in this object.
   *
   * For example:
   *
   * ```ts
   * { prefix: `book-index-`, ... }
   * ```
   *
   * will cause indexes named `book-index-1` and `book-index-foo` to match.
   */
  prefix: string;

  /**
   * List of field configurations for the index.
   *
   * Documents must have fields that match the fields specified in this configuration in order to be indexed.
   * Fields which are not included in the list of fields will be ignored.
   *
   * @example
   * String text field config:
   *
   * ```ts
   * {
   *   name: "isbn",
   *   kind: "text",
   *   // Note "STRING" here which indexes the field as one string (e.g. no splitting).
   *   flags: ["STRING"]
   * }
   * ```
   *
   * @example
   * Full-text text field config:
   *
   * ```ts
   * {
   *   name: "description",
   *   kind: "text",
   *   // Note "TEXT" flag which indexes the field as a full-text field splitting on characters such as spaces.
   *   flags: ["TEXT"]
   * }
   * ```
   */
  fields: IndexFieldConfig[];
}

export interface PatheryConfig {
  /**
   * List of index configurations.
   */
  indexes: IndexConfig[];
}
