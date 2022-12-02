import { App } from "aws-cdk-lib";
import { PatheryStack } from "@pathery/cdk";
import { TestDataStack } from "./test-data-stack";

const app = new App();

const pathery = new PatheryStack(app, "pathery-dev", {
  config: {
    indexes: [
      {
        prefix: "libgen-index-v2",
        fields: [
          {
            name: "title",
            flags: ["TEXT"],
            kind: "text",
          },
          {
            name: "identifier",
            flags: ["STRING"],
            kind: "text",
          },
          {
            name: "year",
            flags: ["INDEXED"],
            kind: "i64",
          },
          {
            name: "author",
            flags: ["TEXT"],
            kind: "text",
          },
          {
            name: "publisher",
            flags: ["TEXT"],
            kind: "text",
          },
          {
            name: "description",
            flags: ["TEXT"],
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
