import { expect, test } from "vitest";
import { dataKey, isVersion } from "../../worker/keys.ts";

test("only paths the pipeline writes map to bucket keys", () => {
  expect(dataKey("v2026-11-15/series/wdi-sp-pop-totl.bin")).toBe(
    "v2026-11-15/series/wdi-sp-pop-totl.bin",
  );
  expect(dataKey("v2026-11-15-2/index.json")).toBe("v2026-11-15-2/index.json");
  for (const path of [
    "current.json",
    "v2026-11-15/current.json",
    "v2026-11-15/series/../../current.json",
    "v2026-11-15/series/a/b.bin",
    "v2026-11-15/series/UPPER.bin",
    "v2026-11-15/series/.bin",
    "v2026-11-15/series/a.json",
    "2026-11-15/series/a.bin",
    "v2026-11-15/series/a.bin/",
    "",
  ]) {
    expect(dataKey(path), path).toBeUndefined();
  }
});

test("a version is a date with an optional run number", () => {
  expect(isVersion("v2026-11-15")).toBe(true);
  expect(isVersion("v2026-11-15-3")).toBe(true);
  expect(isVersion("v2026-11-15/x")).toBe(false);
  expect(isVersion("latest")).toBe(false);
});
