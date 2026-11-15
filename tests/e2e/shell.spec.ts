import { expect, test } from "@playwright/test";

test("the page loads", async ({ page }, info) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "maps" })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await page.screenshot({ path: info.outputPath("shell.png"), fullPage: true });
});

test("health endpoint answers", async ({ request }) => {
  const res = await request.get("/api/health");
  expect(await res.json()).toEqual({ ok: true });
});
