import { Stack } from "aws-cdk-lib";
import { LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
import { AttributeType, BillingMode, Table } from "aws-cdk-lib/aws-dynamodb";
import { Architecture, Code, Function, Runtime } from "aws-cdk-lib/aws-lambda";
import { Construct } from "constructs";

class RustFunction extends Function {
  constructor(scope: Construct, id: string) {
    super(scope, id, {
      code: Code.fromAsset(
        `node_modules/@internal/handler-${id}/target/lambda/${id}`
      ),
      handler: "default",
      runtime: Runtime.PROVIDED_AL2,
      architecture: Architecture.ARM_64,
    });
  }
}

export class AppStack extends Stack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    const table = new Table(this, "Table", {
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

    const postIndex = new RustFunction(this, "post-index");
    table.grantReadWriteData(postIndex);
    postIndex.addEnvironment("TABLE_NAME", table.tableName);

    const api = new RestApi(this, "PatheryApi");

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));
  }
}
