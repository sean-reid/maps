import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "vitest";
import { Camera, MAX_ZOOM } from "../../src/camera.ts";
import { labelSize, LABEL_MIN_PX } from "../../src/draw.ts";
import { layout, type Topology } from "../../src/geometry.ts";
import { atomAt, bounds, measure } from "../../src/shapes.ts";

const geo = layout(
  JSON.parse(
    readFileSync(resolve(import.meta.dirname, "../../public/geo/atoms.json"), "utf8"),
  ) as Topology,
);
const shapes = measure(geo, geo.points);
const index = (iso: string) => geo.atoms.findIndex((a) => a.iso === iso);
const at = (x: number, y: number) => geo.atoms[atomAt(geo, geo.points, shapes, x, y)]?.iso;

test("areas on the equal-area map rank countries as an atlas does", () => {
  const land = shapes.reduce((sum, s) => sum + s.area, 0);
  const share = (iso: string) => (shapes[index(iso)]?.area ?? 0) / land;
  expect(share("RUS")).toBeGreaterThan(0.11);
  expect(share("RUS")).toBeLessThan(0.135);
  expect(share("BRA")).toBeGreaterThan(0.055);
  expect(share("BRA")).toBeLessThan(0.07);
  expect(share("AUS")).toBeGreaterThan(share("IND"));
  expect(share("LUX")).toBeLessThan(0.0001);
});

test("a point resolves to the country drawn there, and the sea to none", () => {
  for (const iso of ["BRA", "IND", "AUS", "DZA", "MNG"]) {
    const anchor = shapes[index(iso)]?.anchor;
    expect(at(anchor?.x ?? 0, anchor?.y ?? 0)).toBe(iso);
  }
  expect(at(2, 2)).toBeUndefined();
});

test("a label anchor lies inside its country, however the country bends", () => {
  for (const iso of ["CHL", "NOR", "VNM", "HRV", "SOM", "JPN", "IDN", "USA", "MYS"]) {
    const anchor = shapes[index(iso)]?.anchor;
    expect(at(anchor?.x ?? 0, anchor?.y ?? 0), iso).toBe(iso);
  }
});

test("an enclave wins over the country around it", () => {
  const lesotho = shapes[index("LSO")]?.anchor;
  expect(at(lesotho?.x ?? 0, lesotho?.y ?? 0)).toBe("LSO");
});

test("the camera fits the map, round-trips points, and keeps the map on screen", () => {
  const camera = new Camera();
  const map = bounds(shapes);
  camera.fit(map, 800, 400);
  const [left] = camera.toScreen(map.x0, map.y0);
  const [right] = camera.toScreen(map.x1, map.y1);
  expect(left).toBeGreaterThanOrEqual(8 - 1e-9);
  expect(right).toBeLessThanOrEqual(792 + 1e-9);

  const corner = camera.toGrid(0, 0);
  const far = camera.toGrid(800, 400);
  const before = camera.toGrid(300, 150);
  camera.zoomAt(300, 150, 3);
  const after = camera.toGrid(300, 150);
  expect(after[0]).toBeCloseTo(before[0], 9);
  expect(after[1]).toBeCloseTo(before[1], 9);
  expect(camera.toScreen(...after)[0]).toBeCloseTo(300, 9);

  camera.zoomAt(0, 0, 1000);
  expect(camera.zoom).toBe(MAX_ZOOM);
  camera.panBy(1e6, 1e6);
  expect(camera.toGrid(0, 0)[0]).toBeCloseTo(corner[0], 9);
  expect(camera.toGrid(0, 0)[1]).toBeCloseTo(corner[1], 9);
  camera.panBy(-1e6, -1e6);
  expect(camera.toGrid(800, 400)[0]).toBeCloseTo(far[0], 9);
  expect(camera.toGrid(800, 400)[1]).toBeCloseTo(far[1], 9);
  camera.zoomAt(400, 200, 1e-6);
  expect(camera.zoom).toBe(1);
  expect(camera.toScreen(map.x0, map.y0)[0]).toBeCloseTo(left, 9);
});

test("a label shows only where the name fits", () => {
  expect(labelSize("India", 200, 200, 30000)).toBeGreaterThanOrEqual(LABEL_MIN_PX);
  expect(labelSize("India", 20, 20, 300)).toBe(0);
  expect(labelSize("United States of America", 90, 400, 30000)).toBe(0);
  expect(labelSize("Chile", 300, 12, 3000)).toBe(0);
  expect(labelSize("Russia", 600, 80, 1500)).toBe(0);
});
