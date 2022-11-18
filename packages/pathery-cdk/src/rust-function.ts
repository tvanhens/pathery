import {
  Architecture,
  Code,
  Function,
  FunctionProps,
  Runtime,
} from "aws-cdk-lib/aws-lambda";
import { RetentionDays } from "aws-cdk-lib/aws-logs";
import { Construct } from "constructs";
import * as path from "path";

export class RustFunction extends Function {
  constructor(scope: Construct, id: string, props?: Partial<FunctionProps>) {
    const lambdaAssetPath = path.join(__dirname, "..", "target", id);
    super(scope, id, {
      ...props,
      code: Code.fromAsset(lambdaAssetPath),
      handler: "default",
      runtime: Runtime.PROVIDED_AL2,
      architecture: Architecture.ARM_64,
      logRetention: RetentionDays.THREE_DAYS,
    });
  }
}
