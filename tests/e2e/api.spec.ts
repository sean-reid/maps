import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "@playwright/test";
import { decodeSeries } from "../../src/series.ts";

const fixture = (path: string) =>
  readFileSync(resolve(import.meta.dirname, "../fixtures/data", path));

test("the index names the data version and lists the series", async ({ request }) => {
  const res = await request.get("/api/index");
  expect(res.status()).toBe(200);
  expect(res.headers()["x-data-version"]).toBe("v2026-01-01");
  expect(res.headers()["cache-control"]).toContain("max-age=300");
  expect(await res.json()).toEqual(JSON.parse(fixture("index.json").toString()));
});

test("a series comes back byte for byte, decodes, and may be cached forever", async ({
  request,
}) => {
  const res = await request.get("/api/data/v2026-01-01/series/wdi-sp-pop-totl.bin");
  expect(res.status()).toBe(200);
  expect(res.headers()["cache-control"]).toContain("immutable");
  const body = await res.body();
  expect(body.equals(fixture("series/wdi-sp-pop-totl.bin"))).toBe(true);
  const series = decodeSeries(new Uint8Array(body).buffer);
  expect(series.codes).toContain("IND");
  expect(series.firstYear).toBe(1960);
  const india = series.codes.indexOf("IND");
  expect(series.values[india * series.years + (2020 - 1960)]).toBeGreaterThan(1.3e9);
});

test("anything else in the bucket stays out of reach", async ({ request }) => {
  for (const path of [
    "/api/data/current.json",
    "/api/data/v2026-01-01/series/nope.bin",
    "/api/data/v2026-01-01/series/..%2F..%2Fcurrent.json",
    "/api/data/v1999-01-01/index.json",
    "/api/nope",
  ]) {
    expect((await request.get(path)).status(), path).toBe(404);
  }
  expect((await request.post("/api/index")).status()).toBe(405);
});
