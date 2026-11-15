import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { Camera } from "../../src/camera.ts";
import { layout, type Topology } from "../../src/geometry.ts";
import { bounds, measure } from "../../src/shapes.ts";

const geo = layout(
  JSON.parse(
    readFileSync(resolve(import.meta.dirname, "../../public/geo/atoms.json"), "utf8"),
  ) as Topology,
);
const shapes = measure(geo, geo.points);

async function open(page: Page) {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  const box = await page.locator("#map").boundingBox();
  if (!box) throw new Error("no map");
  return box;
}

function screenOf(iso: string, box: { x: number; y: number; width: number; height: number }) {
  const camera = new Camera();
  camera.fit(bounds(shapes), box.width, box.height);
  const anchor = shapes[geo.atoms.findIndex((a) => a.iso === iso)]?.anchor;
  const [x, y] = camera.toScreen(anchor?.x ?? 0, anchor?.y ?? 0);
  return { x: box.x + x, y: box.y + y };
}

test("the map fills the page between the title and the footer", async ({ page }, info) => {
  const box = await open(page);
  const viewport = page.viewportSize();
  expect(box.width).toBe(viewport?.width);
  expect(box.height).toBeGreaterThan(200);
  await expect(page.getByRole("heading", { name: "Land area" })).toBeVisible();
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
  await expect(page.locator("#readout")).toContainText(/6\.\d% ?of land outside Antarctica/);
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

test("health endpoint answers", async ({ request }) => {
  const res = await request.get("/api/health");
  expect(await res.json()).toEqual({ ok: true });
});
