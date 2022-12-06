import { CfnOutput, Stack } from "aws-cdk-lib";
import { Bucket } from "aws-cdk-lib/aws-s3";
import { Construct } from "constructs";

export class TestDataStack extends Stack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    const dataBucket = new Bucket(this, "DataBucket");

    new CfnOutput(this, "DataBucketName", {
      value: dataBucket.bucketName,
    });
  }
}
