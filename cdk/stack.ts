import { CfnOutput, Stack } from "aws-cdk-lib";
import { AttributeType, BillingMode, Table } from "aws-cdk-lib/aws-dynamodb";
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

    new CfnOutput(this, "TestTableName", {
      value: table.tableName,
    });
  }
}
