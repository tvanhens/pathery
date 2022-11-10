import { Stack } from "aws-cdk-lib";
import { LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
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

    const postIndex = new RustFunction(this, "post-index");

    const api = new RestApi(this, "PatheryApi");

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));
  }
}
