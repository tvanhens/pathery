import { Stack, aws_lambda } from "aws-cdk-lib";
import { LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
import {
  GatewayVpcEndpointAwsService,
  SubnetType,
  Vpc,
} from "aws-cdk-lib/aws-ec2";
import { FileSystem } from "aws-cdk-lib/aws-efs";
import { Function, LayerVersion } from "aws-cdk-lib/aws-lambda";
import { Architecture, Code, Runtime } from "aws-cdk-lib/aws-lambda";
import { SqsEventSource } from "aws-cdk-lib/aws-lambda-event-sources";
import { IQueue, Queue } from "aws-cdk-lib/aws-sqs";
import { Construct } from "constructs";
import { PatheryConfig } from "./config";
import * as fs from "fs";
import { RustFunction } from "./rust-function";
import { PatheryDashboard } from "./pathery-dashboard";
import { Bucket, IBucket } from "aws-cdk-lib/aws-s3";

export interface PatheryStackProps {
  config: PatheryConfig;
}

export class PatheryStack extends Stack {
  private bucket: IBucket;

  private indexWriterQueue: IQueue;

  constructor(scope: Construct, id: string, props: PatheryStackProps) {
    super(scope, id);

    this.bucket = new Bucket(this, "DataBucket");

    this.indexWriterQueue = new Queue(this, "IndexWriterQueue", {
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

    vpc.addGatewayEndpoint("S3Endpoint", {
      service: GatewayVpcEndpointAwsService.S3,
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
    this.indexWriterProducer(postIndex);

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
    this.indexWriterProducer(deleteDoc);

    const api = new RestApi(this, "PatheryApi");

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));

    const queryActionRoute = indexSingleRoute.addResource("query");

    queryActionRoute.addMethod("POST", new LambdaIntegration(queryIndex));

    const documentRoute = indexSingleRoute.addResource("doc");

    const documentSingleRoute = documentRoute.addResource("{doc_id}");

    documentSingleRoute.addMethod("DELETE", new LambdaIntegration(deleteDoc));

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
      new SqsEventSource(this.indexWriterQueue, {
        batchSize: 10,
      })
    );
    this.bucket.grantRead(indexWriterWorker);
    this.bucket.grantDelete(indexWriterWorker);
    indexWriterWorker.addEnvironment(
      "DATA_BUCKET_NAME",
      this.bucket.bucketName
    );

    new PatheryDashboard(this, "Dashboard", {
      indexWriterWorker,
    });
  }

  private indexWriterProducer(lambda: Function) {
    this.bucket.grantWrite(lambda);
    lambda.addEnvironment("DATA_BUCKET_NAME", this.bucket.bucketName);

    this.indexWriterQueue.grantSendMessages(lambda);
    lambda.addEnvironment(
      "INDEX_WRITER_QUEUE_URL",
      this.indexWriterQueue.queueUrl
    );
  }
}
