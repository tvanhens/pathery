import { CfnOutput, Stack } from "aws-cdk-lib";
import { ApiKey, RestApi } from "aws-cdk-lib/aws-apigateway";
import { Bucket } from "aws-cdk-lib/aws-s3";
import { Construct } from "constructs";

export interface TestDataStackProps {
  patheryApi: RestApi;

  apiKey: ApiKey;
}

export class TestDataStack extends Stack {
  constructor(scope: Construct, id: string, props: TestDataStackProps) {
    super(scope, id);

    const dataBucket = new Bucket(this, "DataBucket");

    new CfnOutput(this, "DataBucketName", {
      value: dataBucket.bucketName,
    });
  }
}
