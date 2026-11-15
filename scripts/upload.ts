// Uploads a pipeline output directory to R2 under a new version, then moves the pointer to it.
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { isVersion } from "../worker/keys.ts";
import { contentType, listFiles } from "./files.ts";

const BUCKET = "maps";
const CONCURRENCY = 6;
const ATTEMPTS = 5;

const [dirArg, version = ""] = process.argv.slice(2);
const token = process.env.CLOUDFLARE_API_TOKEN;
const account = process.env.CLOUDFLARE_ACCOUNT_ID;
if (!dirArg || !isVersion(version) || !token || !account) {
  console.error(
    "usage: upload.ts <dir> <vYYYY-MM-DD[-n]> with CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID set",
  );
  process.exit(2);
}
const dir = resolve(dirArg);
const base = `https://api.cloudflare.com/client/v4/accounts/${account}/r2/buckets/${BUCKET}/objects`;

async function put(key: string, body: Buffer): Promise<void> {
  for (let attempt = 1; ; attempt++) {
    const res = await fetch(`${base}/${key.split("/").map(encodeURIComponent).join("/")}`, {
      method: "PUT",
      headers: { authorization: `Bearer ${token}`, "content-type": contentType(key) },
      body: new Uint8Array(body),
    });
    if (res.ok) return;
    const retryable = res.status === 429 || res.status >= 500;
    if (!retryable || attempt === ATTEMPTS) {
      throw new Error(`${key}: ${res.status} ${(await res.text()).slice(0, 200)}`);
    }
    const wait = Number(res.headers.get("retry-after")) || 2 ** attempt;
    await new Promise((done) => setTimeout(done, wait * 1000));
  }
}

const files = listFiles(dir);
if (!files.includes("index.json") || !files.some((f) => f.startsWith("series/"))) {
  throw new Error(`${dir} does not look like pipeline output`);
}
let next = 0;
await Promise.all(
  Array.from({ length: CONCURRENCY }, async () => {
    while (next < files.length) {
      const path = files[next++] ?? "";
      await put(`${version}/${path}`, readFileSync(join(dir, path)));
    }
  }),
);
await put("current.json", Buffer.from(JSON.stringify({ version })));
console.log(`uploaded ${files.length} files as ${version}`);
