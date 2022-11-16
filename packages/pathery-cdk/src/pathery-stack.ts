import { Stack, aws_lambda } from "aws-cdk-lib";
import { LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
import { SubnetType, Vpc } from "aws-cdk-lib/aws-ec2";
import { FileSystem } from "aws-cdk-lib/aws-efs";
import { FunctionProps, LayerVersion } from "aws-cdk-lib/aws-lambda";
import { Architecture, Code, Function, Runtime } from "aws-cdk-lib/aws-lambda";
import { SqsEventSource } from "aws-cdk-lib/aws-lambda-event-sources";
import { Queue } from "aws-cdk-lib/aws-sqs";
import { Construct } from "constructs";
import * as path from "path";

class RustFunction extends Function {
  constructor(scope: Construct, id: string, props?: Partial<FunctionProps>) {
    const lambdaAssetPath = path.join(__dirname, "..", "target", id);
    super(scope, id, {
      ...props,
      code: Code.fromAsset(lambdaAssetPath),
      handler: "default",
      runtime: Runtime.PROVIDED_AL2,
      architecture: Architecture.ARM_64,
    });
  }
}

export class PatheryStack extends Stack {
  constructor(scope: Construct, id: string) {
    super(scope, id);

    const indexWriterQueue = new Queue(this, "IndexWriterQueue", {
      fifo: true,
      contentBasedDeduplication: true,
    });

    const vpc = new Vpc(this, "Vpc", {
      subnetConfiguration: [
        {
          cidrMask: 28,
          name: "isolated",
          subnetType: SubnetType.PRIVATE_ISOLATED,
        },
      ],
    });

    const efs = new FileSystem(this, "Filesystem", {
      vpc,
    });

    let accessPoint = efs.addAccessPoint("ReadWrite", {
      createAcl: {
        ownerGid: "1001",
        ownerUid: "1001",
        permissions: "750",
      },
      posixUser: {
        uid: "1001",
        gid: "1001",
      },
      path: "/pathery-data",
    });

    let configLayer = new LayerVersion(this, "config-layer", {
      code: Code.fromAsset("config"),
      compatibleArchitectures: [Architecture.ARM_64],
      compatibleRuntimes: [Runtime.PROVIDED_AL2],
    });

    const postIndex = new RustFunction(this, "post-index");
    postIndex.addLayers(configLayer);
    indexWriterQueue.grantSendMessages(postIndex);
    postIndex.addEnvironment("QUEUE_URL", indexWriterQueue.queueUrl);

    const queryIndex = new RustFunction(this, "query-index", {
      vpc,
      vpcSubnets: {
        subnets: vpc.isolatedSubnets,
      },
      filesystem: aws_lambda.FileSystem.fromEfsAccessPoint(
        accessPoint,
        "/mnt/pathery-data"
      ),
    });
    queryIndex.addLayers(configLayer);

    const deleteDoc = new RustFunction(this, "delete-doc");
    deleteDoc.addLayers(configLayer);
    indexWriterQueue.grantSendMessages(deleteDoc);
    deleteDoc.addEnvironment("QUEUE_URL", indexWriterQueue.queueUrl);

    const indexWriterWorker = new RustFunction(this, "index-writer-worker", {
      vpc,
      vpcSubnets: {
        subnets: vpc.isolatedSubnets,
      },
      filesystem: aws_lambda.FileSystem.fromEfsAccessPoint(
        accessPoint,
        "/mnt/pathery-data"
      ),
    });
    indexWriterWorker.addLayers(configLayer);
    indexWriterWorker.addEventSource(new SqsEventSource(indexWriterQueue));

    const api = new RestApi(this, "PatheryApi");

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));

    const queryActionRoute = indexSingleRoute.addResource("query");

    queryActionRoute.addMethod("POST", new LambdaIntegration(queryIndex));

    const documentRoute = indexSingleRoute.addResource("doc");

    const documentSingleRoute = documentRoute.addResource("{doc_id}");

    documentSingleRoute.addMethod("DELETE", new LambdaIntegration(deleteDoc));
  }
}
