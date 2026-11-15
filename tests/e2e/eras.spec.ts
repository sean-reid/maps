import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { Camera } from "../../src/camera.ts";
import { ERAS, withEras } from "../../src/eras.ts";
import { layout, type Topology } from "../../src/geometry.ts";
import { decodeSeries } from "../../src/series.ts";
import { bounds, measure } from "../../src/shapes.ts";
import { createSolver } from "../../src/solver/glue.ts";
import { rowsFor, valuesAt } from "../../src/values.ts";

const root = resolve(import.meta.dirname, "../..");
const geo = layout(
  JSON.parse(readFileSync(resolve(root, "public/geo/atoms.json"), "utf8")) as Topology,
);
const land = measure(geo, geo.points).map((shape) => shape.area);
const population = decodeSeries(
  new Uint8Array(readFileSync(resolve(root, "tests/fixtures/data/series/wdi-sp-pop-totl.bin")))
    .buffer,
);
const ussr = ERAS.find((era) => era.code === "SUN")?.members ?? [];

/** Population 1985 to 1995, but the Soviet republics report as one USSR row until 1991. */
function soviet() {
  const [firstYear, years] = [1985, 11];
  const codes = [...population.codes, "SUN"];
  const values = new Float32Array(codes.length * years).fill(NaN);
  population.codes.forEach((code, row) => {
    for (let y = 0; y < years; y++) {
      const year = firstYear + y;
      const value =
        population.values[row * population.years + (year - population.firstYear)] ?? NaN;
      if (ussr.includes(code) && year <= 1991) {
        const sum = values[(codes.length - 1) * years + y] ?? NaN;
        values[(codes.length - 1) * years + y] = (Number.isFinite(sum) ? sum : 0) + value;
      } else {
        values[row * years + y] = value;
      }
    }
  });
  return { codes, firstYear, years, values };
}

function encode(series: ReturnType<typeof soviet>): Buffer {
  const header = 12 + 3 * series.codes.length;
  const padded = header + ((4 - (header % 4)) % 4);
  const bytes = Buffer.alloc(padded + 4 * series.values.length);
  bytes.write("MSR1", 0, "latin1");
  bytes.writeUInt16LE(series.codes.length, 4);
  bytes.writeUInt16LE(series.firstYear, 6);
  bytes.writeUInt16LE(series.years, 8);
  series.codes.forEach((code, i) => bytes.write(code, 12 + 3 * i, "latin1"));
  series.values.forEach((value, i) => bytes.writeFloatLE(value, padded + 4 * i));
  return bytes;
}

async function open(page: Page, year: number) {
  const series = soviet();
  await page.route("**/series/wdi-ny-gdp-mktp-cd.bin", (route) =>
    route.fulfill({ status: 200, contentType: "application/octet-stream", body: encode(series) }),
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/wdi-ny-gdp-mktp-cd/${year}`);
  await expect(page.locator("html")).toHaveAttribute("data-year", String(year), {
    timeout: 15_000,
  });
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
  const box = await page.locator("#map").boundingBox();
  if (!box) throw new Error("no map");

  // Solve the same values with the same WASM to know where countries are.
  const solver = await createSolver(readFileSync(resolve(root, "src/solver/cartogram.wasm")), geo);
  const plain = valuesAt(series, rowsFor(series, geo.atoms), year);
  const folded = withEras(series, geo.atoms, land, year, plain);
  const shapes = measure(geo, solver.solve(folded.values).points);
  const camera = new Camera();
  camera.fit(bounds(shapes), box.width, box.height);
  const at = (iso: string) => {
    const anchor = shapes[geo.atoms.findIndex((a) => a.iso === iso)]?.anchor;
    const [x, y] = camera.toScreen(anchor?.x ?? 0, anchor?.y ?? 0);
    return { x: box.x + x, y: box.y + y };
  };
  return { at, folded };
}

test("while the USSR reports as one, its republics are one country on the map", async ({
  page,
}, info) => {
  const { at, folded } = await open(page, 1988);
  const total = folded.totals.get(ERAS.findIndex((era) => era.code === "SUN")) ?? 0;
  expect(total).toBeGreaterThan(2.7e8);
  for (const iso of ["RUS", "KAZ", "UKR"]) {
    const point = at(iso);
    await page.mouse.click(point.x, point.y);
    await expect(page.locator("#readout b")).toHaveText("USSR");
    await expect(page.locator("#readout .num").first()).toHaveText(
      Math.round(total).toLocaleString("en"),
    );
    await page.locator("#map").press("Escape");
  }
  const france = at("FRA");
  await page.mouse.click(france.x, france.y);
  await expect(page.locator("#readout b")).toHaveText("France");
  const russia = at("RUS");
  await page.mouse.click(russia.x, russia.y);
  await page.screenshot({ path: info.outputPath("ussr.png") });
});

test("after it dissolves, each republic is its own country again", async ({ page }, info) => {
  const { at } = await open(page, 1993);
  const kazakhstan = at("KAZ");
  await page.mouse.click(kazakhstan.x, kazakhstan.y);
  await expect(page.locator("#readout b")).toHaveText("Kazakhstan");
  const russia = at("RUS");
  await page.mouse.click(russia.x, russia.y);
  await expect(page.locator("#readout b")).toHaveText("Russia");
  await page.screenshot({ path: info.outputPath("republics.png") });
});
