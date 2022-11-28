import APIGateway from "aws-sdk/clients/apigateway";
import S3 from "aws-sdk/clients/s3";
import http from "axios";
import readline from "node:readline";

const api = new APIGateway();

const s3 = new S3();

function requireVar(name: string): string {
  const found = process.env[name];
  if (!found) {
    throw new Error(`${name} should be set in the environment`);
  }
  return found;
}

export async function getApiKey() {
  const response = await api
    .getApiKey({
      apiKey: requireVar("API_KEY_ID"),
      includeValue: true,
    })
    .promise();

  const value = response.value;

  if (!value) {
    throw new Error("Could not get API key value");
  }

  return value;
}

export async function handler(_event: unknown) {
  const readStream = s3
    .getObject({
      Bucket: requireVar("DATA_BUCKET"),
      Key: "libgen.json",
    })
    .createReadStream();

  const rl = readline.createInterface({
    input: readStream,
    crlfDelay: Infinity,
  });

  const batch_size = JSON.parse(requireVar("BATCH_SIZE"));

  let batchNum = 1;

  const index_id = requireVar("INDEX_NAME");

  const batchUrl = `${requireVar("PATHERY_BASE_URL")}index/${index_id}/batch`;

  const apiKey = await getApiKey();

  let batch: unknown[] = [];

  for await (const line of rl) {
    const next = JSON.parse(line);

    if (!next) {
      break;
    }

    batch.push({
      ...(next.id && { __id: `libgen_${next.id}` }),
      ...(next.title && { title: next.title }),
      ...(next.identifier && { identifier: next.identifier }),
      ...(next.author && { author: next.author }),
      ...(next.publisher && { publisher: next.publisher }),
      ...(next.descr && { descr: next.descr }),
      ...(next.year &&
        !isNaN(Number.parseInt(next.year)) && {
          year: Number.parseInt(next.year),
        }),
    });

    if (batch.length >= batch_size) {
      console.log(`Uploading batch #${batchNum++}`);

      await http.post(batchUrl, batch, {
        headers: {
          "Content-Type": "application/json",
          "X-Api-Key": apiKey,
        },
      });

      batch = [];
    }
  }

  await http.post(batchUrl, batch, {
    headers: {
      "Content-Type": "application/json",
      "X-Api-Key": requireVar("API_KEY"),
    },
  });

  console.log("Done");
}
