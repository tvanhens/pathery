export interface TextFieldConfig {
  name: string;
  kind: "text";
  flags: ("STORED" | "STRING" | "TEXT")[];
}

export type IndexFieldConfig = TextFieldConfig;

export interface IndexConfig {
  prefix: string;
  fields: IndexFieldConfig[];
}

export interface PatheryConfig {
  indexes: IndexConfig[];
}
