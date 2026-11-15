import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "vitest";
import { GRID, layout, type Topology } from "../../src/geometry.ts";

const topology = JSON.parse(
  readFileSync(resolve(import.meta.dirname, "../../public/geo/atoms.json"), "utf8"),
) as Topology;
const geo = layout(topology);

function ringArea(k: number): number {
  const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
  let a = 0;
  for (let i = s, j = e - 1; i < e; j = i++) {
    a += (geo.points[2 * j] ?? 0) * (geo.points[2 * i + 1] ?? 0);
    a -= (geo.points[2 * i] ?? 0) * (geo.points[2 * j + 1] ?? 0);
  }
  return a / 2;
}

test("every country Natural Earth has at 1:50m is an atom, once", () => {
  const codes = geo.atoms.map((a) => a.iso);
  expect(new Set(codes).size).toBe(codes.length);
  expect(codes.length).toBeGreaterThan(230);
  for (const iso of ["USA", "FRA", "NOR", "SGP", "BHR", "MLT", "TUV", "AUS"]) {
    expect(codes).toContain(iso);
  }
  expect(codes).not.toContain("ATA");
});

test("the map sits inside the grid with sea around it", () => {
  let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
  for (let i = 0; i < geo.points.length; i += 2) {
    x0 = Math.min(x0, geo.points[i] ?? 0);
    x1 = Math.max(x1, geo.points[i] ?? 0);
    y0 = Math.min(y0, geo.points[i + 1] ?? 0);
    y1 = Math.max(y1, geo.points[i + 1] ?? 0);
  }
  expect(x0).toBeGreaterThanOrEqual(GRID.width * GRID.pad - 1e-6);
  expect(x1).toBeLessThanOrEqual(GRID.width * (1 - GRID.pad) + 1e-6);
  expect(y0).toBeGreaterThan(GRID.height * GRID.pad - 1e-6);
  expect(y1).toBeLessThan(GRID.height * (1 - GRID.pad) + 1e-6);
});

test("no edge is longer than one cell and no ring is degenerate", () => {
  for (let k = 0; k < geo.ringAtom.length; k++) {
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    expect(e - s).toBeGreaterThanOrEqual(3);
    for (let i = s, j = e - 1; i < e; j = i++) {
      const dx = (geo.points[2 * i] ?? 0) - (geo.points[2 * j] ?? 0);
      const dy = (geo.points[2 * i + 1] ?? 0) - (geo.points[2 * j + 1] ?? 0);
      expect(Math.hypot(dx, dy)).toBeLessThanOrEqual(GRID.maxEdge + 1e-9);
    }
    expect(Math.abs(ringArea(k))).toBeGreaterThan(0);
  }
});

test("neighbours share their border points exactly", () => {
  const owners = new Map<string, Set<number>>();
  for (let k = 0; k < geo.ringAtom.length; k++) {
    for (let i = geo.ringStart[k] ?? 0; i < (geo.ringStart[k + 1] ?? 0); i++) {
      const key = `${geo.points[2 * i]},${geo.points[2 * i + 1]}`;
      const set = owners.get(key) ?? new Set<number>();
      set.add(geo.ringAtom[k] ?? -1);
      owners.set(key, set);
    }
  }
  const index = (iso: string) => geo.atoms.findIndex((a) => a.iso === iso);
  const shared = (a: string, b: string) =>
    [...owners.values()].filter((s) => s.has(index(a)) && s.has(index(b))).length;
  expect(shared("FRA", "DEU")).toBeGreaterThan(3);
  expect(shared("USA", "CAN")).toBeGreaterThan(50);
  expect(shared("ZAF", "LSO")).toBeGreaterThan(3);
  expect(shared("FRA", "JPN")).toBe(0);
});

test("land covers a plausible share of the padded map", () => {
  let land = 0;
  for (let k = 0; k < geo.ringAtom.length; k++) land += ringArea(k);
  const frame = GRID.width * GRID.height * (1 - 2 * GRID.pad) ** 2;
  expect(Math.abs(land) / frame).toBeGreaterThan(0.15);
  expect(Math.abs(land) / frame).toBeLessThan(0.45);
});
