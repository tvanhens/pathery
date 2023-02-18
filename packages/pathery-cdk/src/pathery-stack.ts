import {
  Stack,
  aws_lambda,
  CfnOutput,
  Duration,
  StackProps,
} from "aws-cdk-lib";
import {
  ApiKey,
  EndpointType,
  LambdaIntegration,
  RestApi,
} from "aws-cdk-lib/aws-apigateway";
import {
  GatewayVpcEndpointAwsService,
  InterfaceVpcEndpointAwsService,
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
import {
  AttributeType,
  BillingMode,
  ITable,
  Table,
} from "aws-cdk-lib/aws-dynamodb";

export interface PatheryStackProps extends StackProps {
  config: PatheryConfig;

  /**
   * IndexWriter configuration overrides.
   */
  indexWriter?: {
    /**
     * IndexWriter Lambda memorySize.
     *
     * @default 2048
     */
    memorySize?: number;

    /**
     * IndexWriter Lambda timeout duration.
     *
     * @default Duration.minutes(1)
     */
    timeout?: Duration;
  };

  /**
   * QueryHandler configuration overrides.
   */
  queryHandler?: {
    /**
     * IndexWriter Lambda memorySize.
     *
     * @default 3008
     */
    memorySize?: number;
  };
}

export class PatheryStack extends Stack {
  readonly apiKey: ApiKey;

  readonly apiGateway: RestApi;

  private readonly table: ITable;

  private indexWriterQueue: IQueue;

  private deleteQueue: IQueue;

  constructor(scope: Construct, id: string, props: PatheryStackProps) {
    super(scope, id, props);

    this.table = new Table(this, "DataTable", {
      billingMode: BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: "pk",
        type: AttributeType.STRING,
      },
      sortKey: {
        name: "sk",
        type: AttributeType.STRING,
      },
      timeToLiveAttribute: "__ttl",
    });

    this.deleteQueue = new Queue(this, "DeleteQueue", {
      deliveryDelay: Duration.minutes(15),
      visibilityTimeout: Duration.minutes(2),
    });

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
    vpc.addGatewayEndpoint("DynamoEndpoint", {
      service: GatewayVpcEndpointAwsService.DYNAMODB,
    });
    const sqsEndpoint = vpc.addInterfaceEndpoint("SqsGateway", {
      service: InterfaceVpcEndpointAwsService.SQS,
    });
    sqsEndpoint.connections.allowDefaultPortFromAnyIpv4();
    const lambdaEndpoint = vpc.addInterfaceEndpoint("LambdaEndpoint", {
      service: InterfaceVpcEndpointAwsService.LAMBDA,
    });
    lambdaEndpoint.connections.allowDefaultPortFromAnyIpv4();

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

    const batchIndex = new RustFunction(this, "batch-index");
    batchIndex.addLayers(configLayer);
    this.indexWriterProducer(batchIndex);

    const queryIndexPartition = new RustFunction(
      this,
      "query-index-partition-fn",
      {
        memorySize: props.queryHandler?.memorySize ?? 3008,
        timeout: Duration.seconds(5),
        vpc,
        vpcSubnets: {
          subnets: vpc.isolatedSubnets,
        },
        filesystem: aws_lambda.FileSystem.fromEfsAccessPoint(
          accessPoint,
          "/mnt/pathery-data"
        ),
      }
    );
    queryIndexPartition.addLayers(configLayer);
    this.table.grantReadData(queryIndexPartition);
    queryIndexPartition.addEnvironment("DATA_TABLE_NAME", this.table.tableName);
    queryIndexPartition.addEnvironment(
      "ASYNC_DELETE_QUEUE_URL",
      this.deleteQueue.queueUrl
    );

    const queryIndex = new RustFunction(this, "query-index", {
      memorySize: props.queryHandler?.memorySize ?? 3008,
      timeout: Duration.seconds(5),
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
    this.table.grantReadData(queryIndex);
    queryIndex.addEnvironment("DATA_TABLE_NAME", this.table.tableName);
    queryIndex.addEnvironment(
      "ASYNC_DELETE_QUEUE_URL",
      this.deleteQueue.queueUrl
    );
    queryIndexPartition.grantInvoke(queryIndex);
    queryIndex.addEnvironment(
      "QUERY_INDEX_PARTITION_NAME",
      queryIndexPartition.functionName
    );

    const statsIndex = new RustFunction(this, "stats-index", {
      vpc,
      vpcSubnets: {
        subnets: vpc.isolatedSubnets,
      },
      filesystem: aws_lambda.FileSystem.fromEfsAccessPoint(
        accessPoint,
        "/mnt/pathery-data"
      ),
    });
    statsIndex.addLayers(configLayer);
    // FIXME: This doesn't actually get used but is required to be
    //        set because of some tangled internal dependencies.
    statsIndex.addEnvironment(
      "ASYNC_DELETE_QUEUE_URL",
      this.deleteQueue.queueUrl
    );

    const deleteDoc = new RustFunction(this, "delete-doc");
    deleteDoc.addLayers(configLayer);
    this.indexWriterProducer(deleteDoc);

    const api = new RestApi(this, "PatheryApi", {
      restApiName: id,
      endpointConfiguration: {
        types: [EndpointType.REGIONAL],
      },
      defaultMethodOptions: {
        apiKeyRequired: true,
      },
    });

    this.apiGateway = api;

    const apiKey = new ApiKey(this, "DefaultApiKey", {});

    const plan = api.addUsagePlan("DefaultPlan", {
      apiStages: [
        {
          api,
          stage: api.deploymentStage,
        },
      ],
    });

    plan.addApiKey(apiKey);

    this.apiKey = apiKey;

    const indexRoute = api.root.addResource("index");

    const indexSingleRoute = indexRoute.addResource("{index_id}");

    indexSingleRoute.addMethod("POST", new LambdaIntegration(postIndex));

    const queryActionRoute = indexSingleRoute.addResource("query");

    queryActionRoute.addMethod("POST", new LambdaIntegration(queryIndex));

    const statsActionRoute = indexSingleRoute.addResource("stats");

    statsActionRoute.addMethod("GET", new LambdaIntegration(statsIndex));

    const batchIndexRoute = indexSingleRoute.addResource("batch");

    batchIndexRoute.addMethod("POST", new LambdaIntegration(batchIndex));

    const documentRoute = indexSingleRoute.addResource("doc");

    const documentSingleRoute = documentRoute.addResource("{doc_id}");

    documentSingleRoute.addMethod("DELETE", new LambdaIntegration(deleteDoc));

    const indexWriterWorker = new RustFunction(this, "index-writer-worker", {
      memorySize: props.indexWriter?.memorySize ?? 2048,
      timeout: props.indexWriter?.timeout ?? Duration.minutes(1),
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
    this.table.grantReadWriteData(indexWriterWorker);
    indexWriterWorker.addEnvironment("DATA_TABLE_NAME", this.table.tableName);
    this.deleteQueue.grantSendMessages(indexWriterWorker);
    indexWriterWorker.addEnvironment(
      "ASYNC_DELETE_QUEUE_URL",
      this.deleteQueue.queueUrl
    );

    const asyncDeleteWorker = new RustFunction(this, "async-delete-worker", {
      memorySize: props.indexWriter?.memorySize ?? 2048,
      timeout: props.indexWriter?.timeout ?? Duration.minutes(1),
      vpc,
      vpcSubnets: {
        subnets: vpc.isolatedSubnets,
      },
      filesystem: aws_lambda.FileSystem.fromEfsAccessPoint(
        accessPoint,
        "/mnt/pathery-data"
      ),
    });
    asyncDeleteWorker.addLayers(configLayer);
    asyncDeleteWorker.addEventSource(
      new SqsEventSource(this.deleteQueue, {
        batchSize: 10,
      })
    );

    new PatheryDashboard(this, "Dashboard", {
      indexWriterWorker,
    });

    new CfnOutput(this, "ApiKeyOutput", {
      value: apiKey.keyId,
    });
  }

  private indexWriterProducer(lambda: Function) {
    this.table.grantWriteData(lambda);
    lambda.addEnvironment("DATA_TABLE_NAME", this.table.tableName);

    this.indexWriterQueue.grantSendMessages(lambda);
    lambda.addEnvironment(
      "INDEX_WRITER_QUEUE_URL",
      this.indexWriterQueue.queueUrl
    );
  }
}
