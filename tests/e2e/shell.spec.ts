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

test("zooming in keeps the pointed country under the pointer", async ({ page }, info) => {
  const box = await open(page);
  const india = screenOf("IND", box);
  await page.mouse.move(india.x, india.y);
  await page.mouse.wheel(0, -900);
  await page.mouse.click(india.x, india.y);
  await expect(page.locator("#readout")).toContainText("India");
  await page.screenshot({ path: info.outputPath("zoomed.png") });
  await page.locator("#map").press("0");
  await page.mouse.click(india.x, india.y);
  await page.mouse.click(india.x, india.y);
  await expect(page.locator("#readout")).toContainText("India");
});

test("health endpoint answers", async ({ request }) => {
  const res = await request.get("/api/health");
  expect(await res.json()).toEqual({ ok: true });
});
