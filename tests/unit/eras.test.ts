import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "vitest";
import { ERAS, neighbours, withEras } from "../../src/eras.ts";
import { layout, type Topology } from "../../src/geometry.ts";
import { measure } from "../../src/shapes.ts";
import { rowsFor, valuesAt } from "../../src/values.ts";

const geo = layout(
  JSON.parse(
    readFileSync(resolve(import.meta.dirname, "../../public/geo/atoms.json"), "utf8"),
  ) as Topology,
);
const land = measure(geo, geo.points).map((s) => s.area);
const index = (iso: string) => geo.atoms.findIndex((a) => a.iso === iso);
const nan = NaN;

// Czechoslovakia reports until 2002, its two successors from 2003, France throughout.
const series = {
  codes: ["CSK", "CZE", "FRA", "SVK"],
  firstYear: 2000,
  years: 5,
  values: Float32Array.from(
    [
      [100, nan, 300, nan, nan],
      [nan, nan, nan, 70, 80],
      [5, 5, 5, 5, 5],
      [nan, nan, nan, 30, 40],
    ].flat(),
  ),
};
const fold = (year: number) =>
  withEras(series, geo.atoms, land, year, valuesAt(series, rowsFor(series, geo.atoms), year));
const csk = ERAS.findIndex((e) => e.code === "CSK");

test("every era's members are countries on the map, and no era code is one", () => {
  const isos = new Set(geo.atoms.map((a) => a.iso));
  for (const era of ERAS) {
    expect(isos.has(era.code), era.code).toBe(false);
    for (const member of era.members) expect(isos.has(member), member).toBe(true);
  }
});

test("an era's figure is shared among its members by land area", () => {
  const { values, era, totals } = fold(2000);
  const [cze, svk] = [values[index("CZE")] ?? 0, values[index("SVK")] ?? 0];
  expect(cze + svk).toBeCloseTo(100, 9);
  expect(cze / svk).toBeCloseTo((land[index("CZE")] ?? 0) / (land[index("SVK")] ?? 1), 9);
  expect(era[index("CZE")]).toBe(csk);
  expect(era[index("SVK")]).toBe(csk);
  expect(era[index("FRA")]).toBe(-1);
  expect(values[index("FRA")]).toBe(5);
  expect(totals.get(csk)).toBe(100);
});

test("an era's own gaps are interpolated like a country's", () => {
  const { values, totals } = fold(2001);
  expect(totals.get(csk)).toBeCloseTo(200, 9);
  expect((values[index("CZE")] ?? 0) + (values[index("SVK")] ?? 0)).toBeCloseTo(200, 9);
});

test("once the members report for themselves the era is gone", () => {
  const { values, era, totals } = fold(2003);
  expect(values[index("CZE")]).toBe(70);
  expect(values[index("SVK")]).toBe(30);
  expect(era[index("CZE")]).toBe(-1);
  expect(totals.size).toBe(0);
});

test("a series without era rows comes back unchanged", () => {
  const plain = { ...series, codes: ["XXA", "CZE", "FRA", "SVK"] };
  const values = valuesAt(plain, rowsFor(plain, geo.atoms), 2000);
  const folded = withEras(plain, geo.atoms, land, 2000, values);
  expect([...folded.values]).toEqual([...values]);
  expect(folded.era.every((e) => e === -1)).toBe(true);
});

test("each border edge knows the country across it, and coast knows none", () => {
  const other = neighbours(geo);
  const across = (iso: string): Map<number, number> => {
    const counts = new Map<number, number>();
    geo.ringAtom.forEach((atom, k) => {
      if (atom !== index(iso)) return;
      for (let i = geo.ringStart[k] ?? 0; i < (geo.ringStart[k + 1] ?? 0); i++) {
        counts.set(other[i] ?? -1, (counts.get(other[i] ?? -1) ?? 0) + 1);
      }
    });
    return counts;
  };
  const czech = across("CZE");
  expect([...czech.keys()].map((a) => geo.atoms[a]?.iso ?? "coast").sort()).toEqual(
    ["AUT", "DEU", "POL", "SVK"].sort(),
  );
  expect(czech.get(index("SVK"))).toBe(across("SVK").get(index("CZE")));
  const iceland = across("ISL");
  expect([...iceland.keys()]).toEqual([-1]);
  expect(across("LSO").get(index("ZAF"))).toBeGreaterThan(3);
});
