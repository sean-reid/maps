import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { Camera } from "../../src/camera.ts";
import { layout, type Topology } from "../../src/geometry.ts";
import { decodeSeries } from "../../src/series.ts";
import { bounds, measure, type Shape } from "../../src/shapes.ts";
import { createSolver, type Solver } from "../../src/solver/glue.ts";
import { latestFullYear, rowsFor, valuesAt } from "../../src/values.ts";

const root = resolve(import.meta.dirname, "../..");
const geo = layout(
  JSON.parse(readFileSync(resolve(root, "public/geo/atoms.json"), "utf8")) as Topology,
);
const home = bounds(measure(geo, geo.points));
const population = decodeSeries(
  new Uint8Array(readFileSync(resolve(root, "tests/fixtures/data/series/wdi-sp-pop-totl.bin")))
    .buffer,
);
const rows = rowsFor(population, geo.atoms);
const latest = latestFullYear(population);
let solver: Solver;
const solved = new Map<number, Shape[]>();

// The page solves the fixture population with the same WASM, so the test can know where countries end up.
test.beforeAll(async () => {
  solver = await createSolver(readFileSync(resolve(root, "src/solver/cartogram.wasm")), geo);
});

function shapesAt(year: number): Shape[] {
  let shapes = solved.get(year);
  if (!shapes) {
    shapes = measure(geo, solver.solve(valuesAt(population, rows, year)).points);
    solved.set(year, shapes);
  }
  return shapes;
}

async function open(page: Page, path = "/", motion: "reduce" | "no-preference" = "reduce") {
  await page.emulateMedia({ reducedMotion: motion });
  await page.goto(path);
  // A CI runner compiles the WASM and solves several times slower than a laptop.
  await expect(page.locator("html")).toHaveAttribute("data-series", "wdi-sp-pop-totl", {
    timeout: 15_000,
  });
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
  const box = await page.locator("#map").boundingBox();
  if (!box) throw new Error("no map");
  return box;
}

function screenOf(
  iso: string,
  box: { x: number; y: number; width: number; height: number },
  year = latest,
) {
  const shapes = shapesAt(year);
  const camera = new Camera();
  camera.fit(bounds(shapes), box.width, box.height);
  const anchor = shapes[geo.atoms.findIndex((a) => a.iso === iso)]?.anchor;
  const [x, y] = camera.toScreen(anchor?.x ?? 0, anchor?.y ?? 0);
  return { x: box.x + x, y: box.y + y };
}

/** Ink in the top and bottom rows of the canvas, which a map inside the view leaves blank. */
function edgeInk(page: Page): Promise<number> {
  return page.evaluate(() => {
    const canvas = document.querySelector("#map") as HTMLCanvasElement;
    const ctx = canvas.getContext("2d");
    if (!ctx) return -1;
    const paper = ctx.getImageData(1, canvas.height - 2, 1, 1).data;
    let count = 0;
    for (const y of [0, 1, 2, canvas.height - 3, canvas.height - 2, canvas.height - 1]) {
      const row = ctx.getImageData(0, y, canvas.width, 1).data;
      for (let i = 0; i < row.length; i += 4) {
        if (Math.abs((row[i] ?? 0) - (paper[0] ?? 0)) > 24) count++;
      }
    }
    return count;
  });
}

test("the map fills the page between the title and the footer", async ({ page }, info) => {
  const box = await open(page);
  const viewport = page.viewportSize();
  expect(box.width).toBe(viewport?.width);
  expect(box.height).toBeGreaterThan(200);
  await expect(page.getByRole("heading", { name: "Population, total" })).toBeVisible();
  await expect(page.locator("#detail")).toHaveText("World Bank WDI");
  await expect(page.locator("#year-label")).toHaveText(/^20\d\d$/);
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/20\d\d$/);
  await expect(page.getByRole("link", { name: "source" })).toBeVisible();
  const scroll = await page.evaluate(() => document.documentElement.scrollHeight);
  expect(scroll).toBeLessThanOrEqual(viewport?.height ?? 0);
  await page.screenshot({ path: info.outputPath("map.png") });
});

test("tapping a country shows its name and share, tapping the sea clears it", async ({
  page,
}, info) => {
  const box = await open(page);
  const brazil = screenOf("BRA", box);
  await page.mouse.click(brazil.x, brazil.y);
  await expect(page.locator("#readout")).toContainText("Brazil");
  await expect(page.locator("#readout")).toHaveText(
    /^Brazil 2\d\d,\d{3},\d{3} 2\.\d% of the world$/,
  );
  await page.screenshot({ path: info.outputPath("selected.png") });
  await page.mouse.click(box.x + 3, box.y + box.height - 3);
  await expect(page.locator("#readout")).toBeEmpty();
});

test("zooming in keeps the pointed country still and moves the rest", async ({ page }, info) => {
  const box = await open(page);
  const india = screenOf("IND", box);
  const australia = screenOf("AUS", box);
  await page.mouse.move(india.x, india.y);
  await page.mouse.wheel(0, -900);
  await page.mouse.click(india.x, india.y);
  await expect(page.locator("#readout")).toContainText("India");
  await page.screenshot({ path: info.outputPath("zoomed.png") });
  await page.mouse.click(australia.x, australia.y);
  await expect(page.locator("#readout")).not.toContainText("Australia");
  await page.locator("#map").press("Escape");
  await page.locator("#map").press("0");
  await page.mouse.click(australia.x, australia.y);
  await expect(page.locator("#readout")).toContainText("Australia");
});

test("the keyboard zooms, picks the country at the centre and clears it", async ({ page }) => {
  const box = await open(page);
  const map = page.locator("#map");
  const brazil = screenOf("BRA", box);
  await page.mouse.move(brazil.x, brazil.y);
  await page.mouse.wheel(0, -2000);
  await page.mouse.move(box.x + 2, box.y + 2);
  await map.focus();
  await map.press("Enter");
  const first = await page.locator("#readout b").textContent();
  expect(first).toBeTruthy();
  await expect(page.locator("#readout")).toHaveAttribute("aria-live", "polite");
  await map.press("Escape");
  await expect(page.locator("#readout")).toBeEmpty();
  await map.press("Control+0");
  await map.press("Enter");
  await expect(page.locator("#readout b")).toHaveText(first ?? "");
  await map.press("Escape");
  await map.press("0");
  await map.press("Enter");
  await expect(page.locator("#readout")).not.toContainText(first ?? "");
});

test("a press that jitters a pixel or two still selects", async ({ page }) => {
  const box = await open(page);
  const { x, y } = screenOf("BRA", box);
  await page.mouse.move(x, y);
  await page.mouse.down();
  for (const [dx, dy] of [
    [1, 0],
    [1, 1],
    [-1, 1],
    [-1, -1],
    [1, -1],
    [0, 1],
    [1, 0],
    [0, 0],
  ]) {
    await page.mouse.move(x + (dx ?? 0), y + (dy ?? 0));
  }
  await page.mouse.up();
  await page.mouse.move(box.x + 2, box.y + box.height - 2);
  await expect(page.locator("#readout")).toContainText("Brazil");
});

test("a touch tap pins a country and a second tap lets it go", async ({ page, hasTouch }) => {
  test.skip(!hasTouch, "needs a touch screen");
  const box = await open(page);
  const brazil = screenOf("BRA", box);
  await page.touchscreen.tap(brazil.x, brazil.y);
  await expect(page.locator("#readout")).toContainText("Brazil");
  await expect(page.locator("#readout")).toHaveText(/Brazil \d/);
  await page.touchscreen.tap(brazil.x, brazil.y);
  await expect(page.locator("#readout")).toBeEmpty();
});

test("a link to a series and year opens on that year", async ({ page }) => {
  const box = await open(page, "/wdi-sp-pop-totl/1975");
  await expect(page.locator("#year-label")).toHaveText("1975");
  await expect(page.locator("#year")).toHaveValue("1975");
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/1975$/);
  const china = screenOf("CHN", box, 1975);
  await page.mouse.click(china.x, china.y);
  await expect(page.locator("#readout")).toHaveText(
    /^China 9\d\d,\d{3},\d{3} 2\d\.\d% of the world$/,
  );
});

test("an unknown path opens the default series", async ({ page }) => {
  await open(page, "/no-such-series/1999");
  await expect(page.getByRole("heading", { name: "Population, total" })).toBeVisible();
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/20\d\d$/);
});

test("the year slider steps by keyboard and the readout follows", async ({ page }) => {
  const box = await open(page, "/wdi-sp-pop-totl/2000");
  const india = screenOf("IND", box, 2000);
  await page.mouse.click(india.x, india.y);
  const before = await page.locator("#readout .num").first().textContent();
  await page.locator("#year").focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator("html")).toHaveAttribute("data-year", "1999");
  await expect(page.locator("#year-label")).toHaveText("1999");
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/1999$/);
  await expect(page.locator("#readout")).toContainText("India");
  const after = await page.locator("#readout .num").first().textContent();
  expect(Number(after?.replaceAll(",", ""))).toBeLessThan(Number(before?.replaceAll(",", "")));
});

test("play runs to the last year and stops, and pause holds the year", async ({ page }) => {
  await open(page, "/wdi-sp-pop-totl/1960");
  const play = page.getByRole("button", { name: "Play" });
  await play.click();
  const pause = page.getByRole("button", { name: "Pause" });
  await expect
    .poll(async () => Number(await page.locator("#year-label").textContent()))
    .toBeGreaterThanOrEqual(1962);
  await pause.click();
  const held = await page.locator("#year-label").textContent();
  await page.waitForTimeout(900);
  await expect(page.locator("#year-label")).toHaveText(held ?? "");
  await expect(play).toBeVisible();

  await open(page, "/wdi-sp-pop-totl/2023");
  await page.getByRole("button", { name: "Play" }).click();
  await expect(page.locator("#year-label")).toHaveText("2025");
  await expect(page.getByRole("button", { name: "Play" })).toBeVisible();
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/2025$/);
});

test("pausing and playing again at once does not run the years twice as fast", async ({ page }) => {
  await open(page, "/wdi-sp-pop-totl/1960");
  await page.getByRole("button", { name: "Play" }).click();
  await page.getByRole("button", { name: "Pause" }).click();
  await page.getByRole("button", { name: "Play" }).click();
  const from = Number(await page.locator("#year-label").textContent());
  await page.waitForTimeout(2100);
  const to = Number(await page.locator("#year-label").textContent());
  expect(to - from).toBeGreaterThanOrEqual(3);
  expect(to - from).toBeLessThanOrEqual(7);
});

test("with motion on, a change of year springs and comes to rest where the solver put it", async ({
  page,
}, info) => {
  const box = await open(page, "/", "no-preference");
  const before = Number(await page.locator("#year-label").textContent());
  await page.locator("#year").focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator("html")).toHaveAttribute("data-year", String(before - 1));
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
  expect(await edgeInk(page)).toBe(0);
  const india = screenOf("IND", box, before - 1);
  await page.mouse.click(india.x, india.y);
  await expect(page.locator("#readout")).toContainText("India");
  await page.screenshot({ path: info.outputPath("rest.png") });
});

test("nothing is drawn until the first series is in shape", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  let release = (): void => undefined;
  const held = new Promise<void>((done) => (release = done));
  await page.route("**/api/index", async (route) => {
    await held;
    await route.continue();
  });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.locator("#map")).toBeHidden();
  await expect(page.locator("#title")).toBeEmpty();
  await expect(page.locator("#time")).toBeHidden();
  release();
  await expect(page.locator("#map")).toBeVisible();
  // The map is at rest the moment it appears: it did not spring in from the plain map.
  const restAtReveal = await page.evaluate(
    () =>
      !document.querySelector("#map")?.hasAttribute("data-waiting") &&
      document.documentElement.dataset.rest === "true",
  );
  expect(restAtReveal).toBe(true);
  await expect(page.getByRole("heading", { name: "Population, total" })).toBeVisible();
});

test("if the solver cannot load, the plain map shows with a message", async ({ page }) => {
  await page.route("**/*.wasm", (route) => route.fulfill({ status: 404, body: "" }));
  await page.goto("/");
  await expect(page.locator("#map")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Land area" })).toBeVisible();
  await expect(page.locator("#readout")).toHaveText(/stopped responding/);
  await expect(page.locator("#time")).toBeHidden();
});

test("if the first series cannot load, the plain map shows with a message", async ({ page }) => {
  await page.route("**/series/*.bin", (route) => route.fulfill({ status: 500, body: "" }));
  await page.goto("/");
  await expect(page.locator("#map")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Land area" })).toBeVisible();
  await expect(page.locator("#readout")).toHaveText("wdi-sp-pop-totl did not load (500)");
});

test("the solved map sits wholly inside the view", async ({ page }) => {
  await open(page);
  // Before the view followed the solved map, the far north ran off the top of the canvas.
  expect(await edgeInk(page)).toBe(0);
});

test("without data the page falls back to land area", async ({ page }) => {
  await page.route("**/api/index", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: '{"error":"no data yet"}',
    }),
  );
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.getByRole("heading", { name: "Land area" })).toBeVisible();
  const box = await page.locator("#map").boundingBox();
  if (!box) throw new Error("no map");
  const camera = new Camera();
  camera.fit(home, box.width, box.height);
  const anchor = measure(geo, geo.points)[geo.atoms.findIndex((a) => a.iso === "BRA")]?.anchor;
  const [x, y] = camera.toScreen(anchor?.x ?? 0, anchor?.y ?? 0);
  await page.mouse.click(box.x + x, box.y + y);
  await expect(page.locator("#readout")).toHaveText(/^Brazil 6\.\d% of land outside Antarctica$/);
});

test("health endpoint answers", async ({ request }) => {
  const res = await request.get("/api/health");
  expect(await res.json()).toEqual({ ok: true });
});
