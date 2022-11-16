import { App } from "aws-cdk-lib";
import { PatheryStack } from "@pathery/cdk";

const app = new App();

new PatheryStack(app, "pathery-dev", {
  config: {
    indexes: [
      {
        prefix: "book-index-v1-",
        fields: [
          {
            name: "title",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
          {
            name: "author",
            flags: ["STORED", "TEXT"],
            kind: "text",
          },
        ],
      },
    ],
  },
});
