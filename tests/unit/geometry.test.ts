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
      expect(Math.hypot(dx, dy)).toBeGreaterThan(0);
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

test("a reversed arc gives both neighbours the same points, split the same way", () => {
  // Two unit squares side by side sharing arc 0, which the second walks backwards.
  const small: Topology = {
    transform: { scale: [1, 1], translate: [0, 0] },
    arcs: [
      [
        [1, 0],
        [0, 1],
      ],
      [
        [1, 1],
        [-1, 0],
        [0, -1],
        [1, 0],
      ],
      [
        [1, 0],
        [1, 0],
        [0, 1],
        [-1, 0],
      ],
    ],
    objects: {
      atoms: {
        geometries: [
          { type: "Polygon", arcs: [[0, 1]], properties: { iso: "AAA", name: "A" } },
          { type: "MultiPolygon", arcs: [[[~0, 2]]], properties: { iso: "BBB", name: "B" } },
        ],
      },
    },
  };
  const g = layout(small, { width: 40, height: 20, pad: 0.25, maxEdge: 2.5 });
  expect([...g.ringStart]).toEqual([0, 16, 32]);
  expect([...g.ringAtom]).toEqual([0, 1]);
  const ring = (k: number) => {
    const out: string[] = [];
    for (let i = g.ringStart[k] ?? 0; i < (g.ringStart[k + 1] ?? 0); i++) {
      out.push(`${g.points[2 * i]},${g.points[2 * i + 1]}`);
    }
    return out;
  };
  const [a, b] = [ring(0), ring(1)];
  expect(new Set(a).size).toBe(16);
  expect(a.slice(0, 4)).toEqual(["20,15", "20,12.5", "20,10", "20,7.5"]);
  expect(b.slice(0, 4)).toEqual(["20,5", "20,7.5", "20,10", "20,12.5"]);
  expect(a.filter((p) => b.includes(p)).sort()).toEqual(
    ["20,5", "20,7.5", "20,10", "20,12.5", "20,15"].sort(),
  );
});

test("a ring that names a missing arc is an error", () => {
  const broken: Topology = {
    transform: { scale: [1, 1], translate: [0, 0] },
    arcs: [
      [
        [0, 0],
        [1, 0],
        [0, 1],
        [-1, -1],
      ],
    ],
    objects: {
      atoms: {
        geometries: [{ type: "Polygon", arcs: [[3]], properties: { iso: "AAA", name: "A" } }],
      },
    },
  };
  expect(() => layout(broken)).toThrow(/missing arc/);
});
