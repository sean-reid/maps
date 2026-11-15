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
  let sum = 0;
  for (let i = 0; i < cold.length; i++) {
    const gap = Math.abs((cold[i] ?? 0) - (warm.points[i] ?? 0));
    worst = Math.max(worst, gap);
    sum += gap;
  }
  // Both satisfy the areas; they may place a vertex a few cells apart on a 512-cell map.
  expect(sum / cold.length).toBeLessThan(0.5);
  expect(worst).toBeLessThan(8);
});

test("no year of either series stalls the solver", () => {
  for (const slug of ["wdi-sp-pop-totl", "wdi-ny-gdp-mktp-cd"]) {
    const bytes = readFileSync(resolve(root, `tests/fixtures/data/series/${slug}.bin`));
    const series = decodeSeries(new Uint8Array(bytes).buffer);
    const seriesRows = rowsFor(series, geo.atoms);
    // 1960 and 2010 took over two seconds each when a microstate caught a cell centre.
    for (const year of [1960, 1961, 1975, 1990, 2010, 2024]) {
      const { truncated, meanError } = solver.solve(valuesAt(series, seriesRows, year));
      expect(truncated, `${slug} ${year}`).toBe(false);
      expect(meanError, `${slug} ${year}`).toBeLessThan(0.03);
    }
  }
});

test("an error raised inside the WASM carries its message, and the solver still works after it", () => {
  const values = valuesAt(population, rows, 2020);
  const start = new Float64Array(geo.points.length).fill(NaN);
  expect(() => solver.solve(values, start)).toThrow(/start must be finite/);
  expect(() => solver.solve(new Float64Array(3))).toThrow(/3 values/);
  expect(solver.solve(values).meanError).toBeLessThan(0.02);
});

test("a geometry the solver refuses is an error at creation", async () => {
  const wasm = readFileSync(resolve(root, "src/solver/cartogram.wasm"));
  await expect(createSolver(wasm, { ...geo, width: 1 })).rejects.toThrow(/smaller than 2x2/);
});

test("gaps are interpolated, edges are not extended, and empty input is safe", () => {
  const nan = NaN;
  const series = {
    codes: ["AAA", "BBB", "XKX"],
    firstYear: 2000,
    years: 5,
    values: Float32Array.of(10, nan, nan, 40, nan, nan, 5, 6, 7, 8, 1, 1, 1, 1, 1),
  };
  const atoms = [
    { iso: "AAA", name: "A" },
    { iso: "BBB", name: "B" },
    { iso: "KOS", name: "Kosovo" },
    { iso: "ZZZ", name: "Z" },
  ];
  const r = rowsFor(series, atoms);
  expect([...r]).toEqual([0, 1, 2, -1]);
  expect([...valuesAt(series, r, 2001)]).toEqual([20, 5, 1, nan]);
  expect([...valuesAt(series, r, 2002)]).toEqual([30, 6, 1, nan]);
  expect([...valuesAt(series, r, 2004)]).toEqual([nan, 8, 1, nan]);
  expect([...valuesAt(series, r, 2000)]).toEqual([10, nan, 1, nan]);
  expect(valuesAt(series, r, 2005).every(Number.isNaN)).toBe(true);
  expect(latestFullYear(series)).toBe(2003);
  const empty = { codes: [], firstYear: 1990, years: 0, values: new Float32Array(0) };
  expect(latestFullYear(empty)).toBe(1990);
  expect(valuesAt(empty, rowsFor(empty, atoms), 1990).every(Number.isNaN)).toBe(true);
});
