import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { resolve } from "node:path";
import { afterEach, expect, test } from "vitest";
import { upload } from "../../scripts/r2.ts";

const dir = resolve(import.meta.dirname, "../fixtures/data");
let server: Server | undefined;

/** A bucket stand-in on a real socket: `answer` picks the status for the nth request to a key. */
async function bucket(answer: (key: string, nth: number) => number | "reset") {
  const stored = new Map<string, number>();
  const seen = new Map<string, number>();
  const auth = new Set<string>();
  server = createServer((req, res) => {
    const key = decodeURIComponent(req.url ?? "").replace(/^\/objects\//, "");
    const nth = (seen.get(key) ?? 0) + 1;
    seen.set(key, nth);
    auth.add(req.headers.authorization ?? "");
    const chunks: Buffer[] = [];
    req.on("data", (c: Buffer) => chunks.push(c));
    req.on("end", () => {
      const status = answer(key, nth);
      if (status === "reset") return void req.socket.destroy();
      if (status === 200) stored.set(key, Buffer.concat(chunks).length);
      res.writeHead(status, status === 429 ? { "retry-after": "0.01" } : {}).end("{}");
    });
  });
  await new Promise<void>((done) => server?.listen(0, "127.0.0.1", done));
  const { port } = server.address() as AddressInfo;
  const target = { base: `http://127.0.0.1:${port}/objects`, token: "t", backoff: () => 0.01 };
  return { stored, seen, auth, target };
}

afterEach(() => server?.close());

test("every file lands under the version and the pointer is written", async () => {
  const b = await bucket(() => 200);
  expect(await upload(b.target, dir, "v2026-02-03-1")).toBe(3);
  expect([...b.stored.keys()].sort()).toEqual([
    "current.json",
    "v2026-02-03-1/index.json",
    "v2026-02-03-1/series/wdi-ny-gdp-mktp-cd.bin",
    "v2026-02-03-1/series/wdi-sp-pop-totl.bin",
  ]);
  expect(b.stored.get("v2026-02-03-1/series/wdi-sp-pop-totl.bin")).toBe(57952);
  expect([...b.auth]).toEqual(["Bearer t"]);
});

test("server errors, rate limits and dropped connections are retried", async () => {
  const b = await bucket((key, nth) => {
    if (key.endsWith("index.json") && nth === 1) return 503;
    if (key.endsWith("pop-totl.bin") && nth === 1) return 429;
    if (key.endsWith("gdp-mktp-cd.bin") && nth === 1) return "reset";
    return 200;
  });
  await upload(b.target, dir, "v2026-02-03");
  expect(b.stored.size).toBe(4);
  expect(b.seen.get("v2026-02-03/index.json")).toBe(2);
  expect(b.seen.get("v2026-02-03/series/wdi-ny-gdp-mktp-cd.bin")).toBe(2);
});

test("a refused file stops the upload before the pointer moves", async () => {
  const b = await bucket((key) => (key.endsWith("pop-totl.bin") ? 403 : 200));
  await expect(upload(b.target, dir, "v2026-02-03")).rejects.toThrow(/pop-totl\.bin: 403/);
  expect(b.seen.get("v2026-02-03/series/wdi-sp-pop-totl.bin")).toBe(1);
  expect(b.stored.has("current.json")).toBe(false);
});

test("a file that keeps failing gives up after the attempt limit", async () => {
  const b = await bucket((key) => (key.endsWith("index.json") ? 500 : 200));
  await expect(upload({ ...b.target, attempts: 3 }, dir, "v2026-02-03")).rejects.toThrow(
    /after 3 attempts/,
  );
  expect(b.seen.get("v2026-02-03/index.json")).toBe(3);
  expect(b.stored.has("current.json")).toBe(false);
});
