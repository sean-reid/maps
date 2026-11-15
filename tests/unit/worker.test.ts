import { expect, test } from "vitest";
import worker from "../../worker/index.ts";

test("health answers ok", async () => {
  const res = worker.fetch(new Request("https://maps.test/api/health"));
  expect(res.status).toBe(200);
  expect(await res.text()).toBe('{"ok":true}');
});

test("unknown api paths are 404", () => {
  expect(worker.fetch(new Request("https://maps.test/api/nope")).status).toBe(404);
});
