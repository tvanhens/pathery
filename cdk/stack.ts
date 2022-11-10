import { Stack } from "aws-cdk-lib";
import { Construct } from "constructs";

export class DevResourcesStack extends Stack {
  constructor(scope: Construct, id: string) {
    super(scope, id, {
      stackName: "pathery-dev",
    });
  }
}
