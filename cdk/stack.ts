import { CfnOutput, Stack } from "aws-cdk-lib";
import { AttributeType, BillingMode, Table } from "aws-cdk-lib/aws-dynamodb";
import { Architecture, Code, Function, Runtime } from "aws-cdk-lib/aws-lambda";
import { Construct } from "constructs";

export class DevResourcesStack extends Stack {
  constructor(scope: Construct, id: string) {
    super(scope, id, {
      stackName: "pathery-dev",
    });

    const table = new Table(this, "TestTable", {
      billingMode: BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: "pk",
        type: AttributeType.STRING,
      },
      sortKey: {
        name: "sk",
        type: AttributeType.STRING,
      },
    });

    new Function(this, "TestFn", {
      code: Code.fromAsset("target/lambda/hello"),
      runtime: Runtime.PROVIDED_AL2,
      handler: "default",
      architecture: Architecture.ARM_64,
    });

    new CfnOutput(this, "TestTableName", {
      value: table.tableName,
    });
  }
}
