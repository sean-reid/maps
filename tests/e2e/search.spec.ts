import { expect, test, type Page } from "@playwright/test";

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
