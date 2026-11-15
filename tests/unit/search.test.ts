import { expect, test } from "vitest";
import type { SeriesMeta } from "../../src/data.ts";
import { prepare, search } from "../../src/search.ts";

const s = (slug: string, title: string, unit = "", entities = 200, first = 1960): SeriesMeta => ({
  slug,
  title,
  unit,
  source: "World Bank WDI",
  first,
  last: 2024,
  entities,
});

const index = prepare([
  s("pop", "Population, total"),
  s("pop-f014", "Population ages 0-14, female"),
  s("urban", "Urban population", "", 210),
  s("gdp", "GDP", "current US$", 205),
  s("gdp-const", "GDP", "constant 2015 US$", 190),
  s("forest", "Forest area", "sq. km", 214, 1990),
  s("co2", "Carbon dioxide (CO2) emissions from Transport (Energy)", "Mt CO2e", 195),
  s("idp", "Internally displaced persons, new displacement associated with disasters", "", 120),
  s("few", "Population in the largest city", "", 60),
]);
const slugs = (query: string, limit?: number) => search(index, query, limit).map((m) => m.slug);

test("the plain word finds the plain series first", () => {
  expect(slugs("population")[0]).toBe("pop");
  expect(slugs("population")).toEqual(expect.arrayContaining(["urban", "pop-f014", "few"]));
  expect(slugs("gdp")).toEqual(["gdp", "gdp-const"]);
});

test("every word must match, in any order, by prefix", () => {
  expect(slugs("urb pop")).toEqual(["urban"]);
  expect(slugs("pop urb")).toEqual(["urban"]);
  expect(slugs("female population")).toEqual(["pop-f014"]);
  expect(slugs("population zebra")).toEqual([]);
});

test("a unit or a fragment inside a word also finds a series", () => {
  expect(slugs("km")).toEqual(["forest"]);
  expect(slugs("constant gdp")).toEqual(["gdp-const"]);
  expect(slugs("placed")).toEqual(["idp"]);
  expect(slugs("ar")).not.toContain("co2");
  expect(slugs("CO2")).toEqual(["co2"]);
});

test("case, accents and punctuation do not matter", () => {
  expect(slugs("  POPULATION,  Total ")).toEqual(["pop"]);
  expect(slugs("Pôpulation")[0]).toBe("pop");
});

test("an empty query lists the broadest series, up to the limit", () => {
  expect(slugs("", 3)).toEqual(["urban", "gdp", "pop"]);
  expect(slugs("")).toHaveLength(9);
  expect(slugs("").at(-1)).toBe("few");
});
