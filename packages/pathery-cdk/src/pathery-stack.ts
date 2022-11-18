import { Stack, aws_lambda } from "aws-cdk-lib";
import { LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
import { SubnetType, Vpc } from "aws-cdk-lib/aws-ec2";
import { FileSystem } from "aws-cdk-lib/aws-efs";
import { LayerVersion } from "aws-cdk-lib/aws-lambda";
import { Architecture, Code, Runtime } from "aws-cdk-lib/aws-lambda";
import { SqsEventSource } from "aws-cdk-lib/aws-lambda-event-sources";
import { Queue } from "aws-cdk-lib/aws-sqs";
import { Construct } from "constructs";
import { PatheryConfig } from "./config";
import * as fs from "fs";
import { RustFunction } from "./rust-function";
import { PatheryDashboard } from "./pathery-dashboard";

export interface PatheryStackProps {
  config: PatheryConfig;
}

export class PatheryStack extends Stack {
  constructor(scope: Construct, id: string, props: PatheryStackProps) {
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

    fs.mkdirSync(".pathery/layer/pathery", { recursive: true });
    fs.writeFileSync(
      ".pathery/layer/pathery/config.json",
      JSON.stringify(props.config)
    );
    let configLayer = new LayerVersion(this, "config-layer", {
      code: Code.fromAsset(".pathery/layer"),
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
    indexWriterWorker.addEventSource(
      new SqsEventSource(indexWriterQueue, {
        batchSize: 10,
      })
    );

    const api = new RestApi(this, "PatheryApi");

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));

    const queryActionRoute = indexSingleRoute.addResource("query");

    queryActionRoute.addMethod("POST", new LambdaIntegration(queryIndex));

    const documentRoute = indexSingleRoute.addResource("doc");

    const documentSingleRoute = documentRoute.addResource("{doc_id}");

    documentSingleRoute.addMethod("DELETE", new LambdaIntegration(deleteDoc));

    new PatheryDashboard(this, "Dashboard", {
      indexWriterWorker,
    });
  }
}
