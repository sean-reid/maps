import { readFileSync } from "node:fs";
import { join } from "node:path";
import { contentType, listFiles } from "./files.ts";

export interface Target {
  /** URL of the bucket's object collection, without a trailing slash. */
  base: string;
  token: string;
  attempts?: number;
  concurrency?: number;
  /** Seconds to wait before the nth retry when the server names none. */
  backoff?: (attempt: number) => number;
}

const MAX_WAIT_S = 60;

async function put(target: Target, key: string, body: Buffer): Promise<void> {
  const attempts = target.attempts ?? 5;
  const backoff = target.backoff ?? ((n: number) => 2 ** n);
  for (let attempt = 1; ; attempt++) {
    let failure: string;
    let wait = backoff(attempt);
    try {
      const res = await fetch(
        `${target.base}/${key.split("/").map(encodeURIComponent).join("/")}`,
        {
          method: "PUT",
          headers: { authorization: `Bearer ${target.token}`, "content-type": contentType(key) },
          body: new Uint8Array(body),
        },
      );
      if (res.ok) return;
      failure = `${res.status} ${(await res.text()).slice(0, 200)}`;
      if (res.status !== 429 && res.status < 500)
        throw new Error(`${key}: ${failure}`, { cause: "final" });
      const asked = Number(res.headers.get("retry-after"));
      if (Number.isFinite(asked) && asked > 0) wait = asked;
    } catch (error) {
      if (error instanceof Error && error.cause === "final") throw error;
      failure ??= error instanceof Error ? error.message : String(error);
    }
    if (attempt === attempts) throw new Error(`${key}: ${failure} after ${attempts} attempts`);
    await new Promise((done) => setTimeout(done, Math.min(wait, MAX_WAIT_S) * 1000));
  }
}

/** Uploads pipeline output as `version`, then points `current.json` at it. Returns the file count. */
export async function upload(target: Target, dir: string, version: string): Promise<number> {
  const files = listFiles(dir);
  if (!files.includes("index.json") || !files.some((f) => f.startsWith("series/"))) {
    throw new Error(`${dir} does not look like pipeline output`);
  }
  let next = 0;
  let failed = false;
  await Promise.all(
    Array.from({ length: target.concurrency ?? 6 }, async () => {
      while (next < files.length && !failed) {
        const path = files[next++] ?? "";
        try {
          await put(target, `${version}/${path}`, readFileSync(join(dir, path)));
        } catch (error) {
          failed = true;
          throw error;
        }
      }
    }),
  );
  await put(target, "current.json", Buffer.from(JSON.stringify({ version })));
  return files.length;
}
