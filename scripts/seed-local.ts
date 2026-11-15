// Fills wrangler's local R2 with the fixture data set, for `wrangler dev` and the e2e run.
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { contentType, listFiles } from "./files.ts";

const VERSION = "v2026-01-01";
const dir = resolve(process.argv[2] ?? resolve(import.meta.dirname, "../tests/fixtures/data"));

function put(key: string, file: string): void {
  execFileSync(
    "npx",
    [
      "wrangler",
      "r2",
      "object",
      "put",
      `maps/${key}`,
      "--local",
      "--file",
      file,
      "--content-type",
      contentType(key),
    ],
    { stdio: ["ignore", "ignore", "inherit"] },
  );
}

for (const path of listFiles(dir)) put(`${VERSION}/${path}`, join(dir, path));
const pointer = join(mkdtempSync(join(tmpdir(), "maps-seed-")), "current.json");
writeFileSync(pointer, JSON.stringify({ version: VERSION }));
put("current.json", pointer);
console.log(`seeded local R2 with ${VERSION} from ${dir}`);
