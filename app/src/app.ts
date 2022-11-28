import { App } from "aws-cdk-lib";
import { PatheryStack } from "@pathery/cdk";
import { TestDataStack } from "./test-data-stack";

const app = new App();

const pathery = new PatheryStack(app, "pathery-dev", {
  config: {
    indexes: [
      {
        prefix: "libgen-index-v1",
        fields: [
          {
            name: "title",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
          {
            name: "identifier",
            flags: ["STORED", "STRING"],
            kind: "text",
          },
          {
            name: "year",
            flags: ["STORED", "INDEXED"],
            kind: "i64",
          },
          {
            name: "author",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
          {
            name: "publisher",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
          {
            name: "descr",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
        ],
      },
    ],
  },
});

new TestDataStack(app, "pathery-test-data", {
  apiKey: pathery.apiKey,
  patheryApi: pathery.apiGateway,
});
