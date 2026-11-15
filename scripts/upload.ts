// Uploads a pipeline output directory to R2 under a new version, then moves the pointer to it.
import { resolve } from "node:path";
import { isVersion } from "../worker/keys.ts";
import { upload } from "./r2.ts";

const [dir, version = ""] = process.argv.slice(2);
const token = process.env.CLOUDFLARE_API_TOKEN;
const account = process.env.CLOUDFLARE_ACCOUNT_ID;
if (!dir || !isVersion(version) || !token || !account) {
  console.error(
    "usage: upload.ts <dir> <vYYYY-MM-DD[-run[-attempt]]> with CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID set",
  );
  process.exit(2);
}
const base = `https://api.cloudflare.com/client/v4/accounts/${account}/r2/buckets/maps/objects`;
const count = await upload({ base, token }, resolve(dir), version);
console.log(`uploaded ${count} files as ${version}`);
