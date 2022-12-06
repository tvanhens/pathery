import * as AWS from "aws-sdk";
import http, { AxiosError } from "axios";
import readline from "node:readline";

const maxBatch = 20_000;
const bucketName = "pathery-test-data-databuckete3889a50-11yc5palv3nvl";
const batchSize = 25;
const patheryEndpoint =
  "https://nlztni8cx5.execute-api.us-east-1.amazonaws.com/prod/";
const index_id = "libgen-index-v2-1";
const apiKeyId = "7xyag5xp0d";

const api = new AWS.APIGateway();

const s3 = new AWS.S3();

export async function getApiKey() {
  const response = await api
    .getApiKey({
      apiKey: apiKeyId,
      includeValue: true,
    })
    .promise();

  const value = response.value;

  if (!value) {
    throw new Error("Could not get API key value");
  }

  return value;
}

async function uploadBatch(apiKey: string, batch: any[]) {
  const batchUrl = `${patheryEndpoint}index/${index_id}/batch`;

  try {
    await http.post(batchUrl, batch, {
      headers: {
        "Content-Type": "application/json",
        "X-Api-Key": apiKey,
      },
    });
    return { status: "OK" as const };
  } catch (err) {
    if (err instanceof AxiosError) {
      if (!err.response) {
        console.error(err);
        process.exit(1);
      }

      const message: string = err.response.data.message;
      const code = err.response.status;

      if (code !== 500) {
        console.error(err);
        process.exit(1);
      }

      console.log(`[${code}] ${message}`);

      return { status: "Error" as const, code, message };
    }
  }
}

export async function* batchGenerator() {
  const readStream = s3
    .getObject({
      Bucket: bucketName,
      Key: "libgen.json",
    })
    .createReadStream();

  const rl = readline.createInterface({
    input: readStream,
    crlfDelay: Infinity,
  });

  let batchNum = 1;

  let batch: unknown[] = [];

  for await (const line of rl) {
    if (batchNum > maxBatch) {
      return batch;
    }
    const next = JSON.parse(line);

    if (!next) {
      break;
    }

    const alreadyInBatch = batch.find(
      ({ __id }: any) => __id === `libgen_${next.id}`
    );

    if (!!alreadyInBatch) {
      continue;
    }

    const formatted = {
      ...(next.id && { __id: `libgen_${next.id}` }),
      ...(next.title && { title: next.title }),
      ...(next.identifier && { identifier: next.identifier }),
      ...(next.author && { author: next.author }),
      ...(next.publisher && { publisher: next.publisher }),
      ...(next.descr && { description: next.descr }),
      ...(next.year &&
        !isNaN(Number.parseInt(next.year)) && {
          year: Number.parseInt(next.year),
        }),
    };

    if (Object.keys(formatted).length <= 1) {
      continue;
    }

    batch.push(formatted);

    if (batch.length >= batchSize) {
      console.log(`Uploading batch #${batchNum++}`);

      yield batch;

      batch = [];
    }
  }

  return batch;
}

async function startUploader(
  apiKey: string,
  batches: AsyncGenerator<unknown[], unknown[], unknown>
) {
  for await (const batch of batches) {
    let attempts = 0;
    while (true) {
      if (attempts >= 3) {
        process.exit(1);
      }

      attempts++;

      const result = await uploadBatch(apiKey, batch);

      if (result?.status === "OK") {
        break;
      }

      console.log("Backing off...");

      await new Promise((resolve) => {
        setTimeout(resolve, 2000);
      });
    }
  }
}

export async function doIndex(numUploader: number) {
  const apiKey = await getApiKey();

  const batches = batchGenerator();

  const uploaderList: Promise<any>[] = [];

  for (let i = 0; i < numUploader; i++) {
    uploaderList.push(startUploader(apiKey, batches));
  }

  await Promise.all(uploaderList);

  console.log("Done");
}

doIndex(10);
