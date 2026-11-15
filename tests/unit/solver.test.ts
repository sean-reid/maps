import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { beforeAll, expect, test } from "vitest";
import { layout, type Topology } from "../../src/geometry.ts";
import { decodeSeries } from "../../src/series.ts";
import { measure } from "../../src/shapes.ts";
import { createSolver, type Solver } from "../../src/solver/glue.ts";
import { latestFullYear, rowsFor, valuesAt } from "../../src/values.ts";

const root = resolve(import.meta.dirname, "../..");
const geo = layout(
  JSON.parse(readFileSync(resolve(root, "public/geo/atoms.json"), "utf8")) as Topology,
);
const file = readFileSync(resolve(root, "tests/fixtures/data/series/wdi-sp-pop-totl.bin"));
const population = decodeSeries(new Uint8Array(file).buffer);
const rows = rowsFor(population, geo.atoms);
const index = (iso: string) => geo.atoms.findIndex((a) => a.iso === iso);
let solver: Solver;

beforeAll(async () => {
  solver = await createSolver(readFileSync(resolve(root, "src/solver/cartogram.wasm")), geo);
});

test("population reaches the map through the series decoder, by country code", () => {
  const values = valuesAt(population, rows, 2020);
  expect(values[index("IND")]).toBeGreaterThan(1.3e9);
  expect(values[index("KOS")]).toBeGreaterThan(1e6);
  expect(values[index("ATF")]).toBeNaN();
  expect(valuesAt(population, rows, 1900).every(Number.isNaN)).toBe(true);
  expect(latestFullYear(population)).toBeGreaterThanOrEqual(2022);
});

test("the WASM solver sizes countries by population", () => {
  const values = valuesAt(population, rows, 2020);
  const { points, meanError, truncated } = solver.solve(values);
  expect(truncated).toBe(false);
  expect(meanError).toBeLessThan(0.02);
  const [before, after] = [measure(geo, geo.points), measure(geo, points)];
  const land = before.reduce((sum, s) => sum + s.area, 0);
  const total = after.reduce((sum, s) => sum + s.area, 0);
  expect(total / land).toBeGreaterThan(0.97);
  expect(total / land).toBeLessThan(1.03);
  const share = (iso: string) => (after[index(iso)]?.area ?? 0) / total;
  expect(share("IND")).toBeGreaterThan(0.15);
  expect(share("IND")).toBeLessThan(0.2);
  expect(share("CHN")).toBeGreaterThan(0.15);
  expect(share("RUS")).toBeLessThan(0.03);
  expect(share("CAN")).toBeLessThan((before[index("CAN")]?.area ?? 0) / land / 4);
});

test("a warm start for the next year lands close to a cold solve", () => {
  const cold2019 = solver.solve(valuesAt(population, rows, 2019)).points;
  const warm = solver.solve(valuesAt(population, rows, 2020), cold2019);
  expect(warm.meanError).toBeLessThan(0.02);
  const cold = solver.solve(valuesAt(population, rows, 2020)).points;
  let worst = 0;
  for (let i = 0; i < cold.length; i++)
    worst = Math.max(worst, Math.abs((cold[i] ?? 0) - (warm.points[i] ?? 0)));
  expect(worst).toBeLessThan(3);
});

test("bad input is an error with the solver's own message", () => {
  expect(() => solver.solve(new Float64Array(3))).toThrow(/3 values/);
});
