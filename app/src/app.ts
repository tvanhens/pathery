import { App } from "aws-cdk-lib";
import { PatheryStack } from "@pathery/cdk";
import { TestDataStack } from "./test-data-stack";

const app = new App();

const pathery = new PatheryStack(app, "pathery-dev", {
  config: {
    indexes: [
      {
        prefix: "test-index-v1",
        fields: [
          {
            name: "author",
            flags: ["TEXT"],
            kind: "text",
          },
          {
            name: "song",
            flags: ["TEXT"],
            kind: "text",
          },
          {
            name: "genre",
            flags: ["STRING"],
            kind: "text",
          },
          {
            name: "releaseDate",
            flags: ["INDEXED"],
            kind: "i64",
          },
        ],
      },
    ],
  },
});

new TestDataStack(app, "pathery-test-data");
