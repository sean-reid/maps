import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "vitest";
import { decodeSeries } from "../../src/series.ts";

const bytes = readFileSync(resolve(import.meta.dirname, "../fixtures/series-v1.bin"));
const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);

test("decodes the fixture the Rust encoder wrote", () => {
  const s = decodeSeries(buffer);
  expect(s.codes).toEqual(["AUS", "BRA", "CHN"]);
  expect(s.firstYear).toBe(1999);
  expect(s.years).toBe(3);
  expect(s.values.length).toBe(9);
  const v = Array.from(s.values);
  expect(v.slice(0, 2)).toEqual([1, 2.5]);
  expect(v[2]).toBeNaN();
  expect(v[3]).toBeNaN();
  expect(v.slice(4, 7)).toEqual([4, 5, 100]);
  expect(v[7]).toBeNaN();
  expect(Object.is(v[8], -0)).toBe(true);
});

test("rejects bad magic", () => {
  const bad = buffer.slice(0);
  new Uint8Array(bad)[0] = 0x58;
  expect(() => decodeSeries(bad)).toThrow(/magic/);
});

test("rejects a buffer shorter than the header declares", () => {
  expect(() => decodeSeries(buffer.slice(0, buffer.byteLength - 4))).toThrow(/truncated/);
  expect(() => decodeSeries(buffer.slice(0, 8))).toThrow(/truncated/);
});
