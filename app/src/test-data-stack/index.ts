import { Duration, Stack } from "aws-cdk-lib";
import { ApiKey, IRestApi, RestApi } from "aws-cdk-lib/aws-apigateway";
import { Architecture } from "aws-cdk-lib/aws-lambda";
import { NodejsFunction } from "aws-cdk-lib/aws-lambda-nodejs";
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

    const reIndexer = new NodejsFunction(this, "re-indexer", {
      memorySize: 1000,
      timeout: Duration.minutes(15),
      architecture: Architecture.ARM_64,
    });
    dataBucket.grantRead(reIndexer);
    props.apiKey.grantRead(reIndexer);
    reIndexer.addEnvironment("DATA_BUCKET", dataBucket.bucketName);
    reIndexer.addEnvironment("INDEX_NAME", "libgen-index-v1");
    reIndexer.addEnvironment("PATHERY_BASE_URL", props.patheryApi.url);
    reIndexer.addEnvironment("BATCH_SIZE", "500");
    reIndexer.addEnvironment("API_KEY_ID", props.apiKey.keyId);
  }
}
