import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { decodeSeries } from "../../src/series.ts";
import { coveredYears } from "../../src/values.ts";

async function open(page: Page, path = "/") {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(path);
  await expect(page.locator("html")).toHaveAttribute("data-series", /./, { timeout: 15_000 });
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
}

async function shows(page: Page, slug: string) {
  await expect(page.locator("html")).toHaveAttribute("data-series", slug);
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
}

test("typing finds a series and Enter puts it on the map", async ({ page }, info) => {
  await open(page, "/wdi-sp-pop-totl/2010");
  const search = page.getByRole("combobox", { name: "Search series" });
  await expect(search).toHaveAttribute("placeholder", "Search 2 series");
  await search.fill("gdp");
  const options = page.getByRole("option");
  await expect(options).toHaveCount(1);
  await expect(options.first()).toHaveText("GDP current US$, 1960 to 2025, 214 countries");
  await page.screenshot({ path: info.outputPath("results.png") });
  await search.press("Enter");
  await shows(page, "wdi-ny-gdp-mktp-cd");
  await expect(page.getByRole("heading", { name: "GDP" })).toBeVisible();
  await expect(page.locator("#detail")).toHaveText("current US$, World Bank WDI");
  await expect(page.locator("#year-label")).toHaveText("2010");
  await expect(page).toHaveURL(/\/wdi-ny-gdp-mktp-cd\/2010$/);
  await expect(page).toHaveTitle("GDP · maps");
  await expect(search).toHaveValue("");
  await expect(page.getByRole("listbox")).toBeHidden();
  await page.screenshot({ path: info.outputPath("gdp.png") });
});

test("the list opens on focus, moves by arrow keys, and closes on Escape", async ({ page }) => {
  await open(page);
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.focus();
  await expect(page.getByRole("option")).toHaveCount(2);
  await expect(search).toHaveAttribute("aria-expanded", "true");
  await search.press("ArrowDown");
  await search.press("ArrowDown");
  const second = page.getByRole("option").nth(1);
  await expect(second).toHaveAttribute("aria-selected", "true");
  await expect(search).toHaveAttribute(
    "aria-activedescendant",
    (await second.getAttribute("id")) ?? "",
  );
  await search.press("Escape");
  await expect(page.getByRole("listbox")).toBeHidden();
  await expect(search).toHaveAttribute("aria-expanded", "false");
});

test("a query nothing matches says so and Enter does nothing", async ({ page }) => {
  await open(page);
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.fill("zebra");
  await expect(page.getByRole("listbox")).toHaveText("No series by that name");
  await expect(page.getByRole("option")).toHaveAttribute("aria-disabled", "true");
  await search.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-series", "wdi-sp-pop-totl");
});

test("clicking a result switches series, and Back returns to the one before", async ({ page }) => {
  await open(page, "/wdi-sp-pop-totl/1990");
  await page.getByRole("combobox", { name: "Search series" }).fill("gd");
  await page.getByRole("option", { name: /^GDP/ }).click();
  await shows(page, "wdi-ny-gdp-mktp-cd");
  await expect(page).toHaveURL(/\/wdi-ny-gdp-mktp-cd\/1990$/);
  await page.goBack();
  await shows(page, "wdi-sp-pop-totl");
  await expect(page.getByRole("heading", { name: "Population, total" })).toBeVisible();
  await expect(page.locator("#year-label")).toHaveText("1990");
  await page.goForward();
  await shows(page, "wdi-ny-gdp-mktp-cd");
});

test("Back straight after a pick keeps the address of the series it returns to", async ({
  page,
}) => {
  await open(page, "/wdi-sp-pop-totl/1990");
  await page.getByRole("combobox", { name: "Search series" }).fill("gdp");
  await page.getByRole("option", { name: /^GDP/ }).click();
  await expect(page.locator("html")).toHaveAttribute("data-series", "wdi-ny-gdp-mktp-cd");
  await page.goBack();
  await shows(page, "wdi-sp-pop-totl");
  await page.waitForTimeout(600);
  await expect(page).toHaveURL(/\/wdi-sp-pop-totl\/1990$/);
});

test("after a pick the keyboard is on the map", async ({ page }) => {
  await open(page);
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.fill("gdp");
  await search.press("Enter");
  await shows(page, "wdi-ny-gdp-mktp-cd");
  await expect(page.locator("#map")).toBeFocused();
});

test("without data the search box stays disabled", async ({ page }) => {
  await page.route("**/api/index", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: '{"error":"no data yet"}',
    }),
  );
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.locator("#search")).toBeDisabled();
});

test("a series that fails to load says so and can be picked again", async ({ page }) => {
  await open(page);
  let fail = true;
  await page.route("**/series/wdi-ny-gdp-mktp-cd.bin", (route) =>
    fail ? route.fulfill({ status: 500, body: "" }) : route.continue(),
  );
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.fill("gdp");
  await search.press("Enter");
  await expect(page.locator("#readout")).toHaveText("wdi-ny-gdp-mktp-cd did not load (500)");
  fail = false;
  await search.fill("gdp");
  await search.press("Enter");
  await shows(page, "wdi-ny-gdp-mktp-cd");
});

test("a map reached by switching series and stepping years matches the one loaded directly", async ({
  page,
}) => {
  const picture = () =>
    page.evaluate(() => (document.querySelector("#map") as HTMLCanvasElement).toDataURL());
  await open(page, "/wdi-ny-gdp-mktp-cd/1994");
  const direct = await picture();

  await open(page, "/wdi-sp-pop-totl/2000");
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.fill("gdp");
  await search.press("Enter");
  await shows(page, "wdi-ny-gdp-mktp-cd");
  await page.locator("#year").focus();
  for (let year = 1999; year >= 1994; year--) {
    await page.keyboard.press("ArrowLeft");
    await expect(page.locator("html")).toHaveAttribute("data-year", String(year));
  }
  await expect(page.locator("html")).toHaveAttribute("data-rest", "true");
  await page.locator("#year").blur();
  await page.mouse.move(2, 2);
  expect(await picture()).toBe(direct);
});

/** A series in the binary format: every country reports from `full` on, only the first three before. */
function sparseSeries(codes: string[], firstYear: number, years: number, full: number): Buffer {
  const header = 12 + 3 * codes.length;
  const padded = header + ((4 - (header % 4)) % 4);
  const bytes = Buffer.alloc(padded + 4 * codes.length * years);
  bytes.write("MSR1", 0, "latin1");
  bytes.writeUInt16LE(codes.length, 4);
  bytes.writeUInt16LE(firstYear, 6);
  bytes.writeUInt16LE(years, 8);
  codes.forEach((code, i) => bytes.write(code, 12 + 3 * i, "latin1"));
  codes.forEach((_, row) => {
    for (let y = 0; y < years; y++) {
      const has = row < 3 || firstYear + y >= full;
      bytes.writeFloatLE(has ? 100 + row + y : NaN, padded + 4 * (row * years + y));
    }
  });
  return bytes;
}

test("years with too few countries are left out of the range, and a carried year falls back", async ({
  page,
}) => {
  const population = decodeSeries(
    new Uint8Array(
      readFileSync(resolve(import.meta.dirname, "../fixtures/data/series/wdi-sp-pop-totl.bin")),
    ).buffer,
  );
  const body = sparseSeries(population.codes.slice(0, 60), 2000, 10, 2004);
  expect(coveredYears(decodeSeries(new Uint8Array(body).buffer))).toEqual({
    first: 2004,
    last: 2009,
    opening: 2009,
  });
  await page.route("**/series/wdi-ny-gdp-mktp-cd.bin", (route) =>
    route.fulfill({ status: 200, contentType: "application/octet-stream", body }),
  );

  await open(page, "/wdi-ny-gdp-mktp-cd/2001");
  await expect(page.locator("#year-label")).toHaveText("2004");
  await expect(page.locator("#year")).toHaveAttribute("min", "2004");
  await expect(page.locator("#year")).toHaveAttribute("max", "2009");
  await expect(page).toHaveURL(/\/wdi-ny-gdp-mktp-cd\/2004$/);

  await open(page, "/wdi-sp-pop-totl/2002");
  const search = page.getByRole("combobox", { name: "Search series" });
  await search.fill("gdp");
  await search.press("Enter");
  await shows(page, "wdi-ny-gdp-mktp-cd");
  await expect(page.locator("#year-label")).toHaveText("2009");
});
