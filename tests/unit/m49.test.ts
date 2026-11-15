import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "vitest";

const root = resolve(import.meta.dirname, "../..");
const atoms = (
  JSON.parse(readFileSync(resolve(root, "public/geo/atoms.json"), "utf8")) as {
    objects: { atoms: { geometries: { properties: { iso: string } }[] } };
  }
).objects.atoms.geometries.map((g) => g.properties.iso);
const [header, ...rows] = readFileSync(resolve(root, "tools/pipeline/data/m49.csv"), "utf8")
  .trimEnd()
  .split("\n");
const table = rows.map((r): [string, string] => {
  const [m49 = "", iso = ""] = r.split(",");
  return [m49, iso];
});

// Natural Earth carries no numeric code for these four.
const WITHOUT_CODE = ["CYN", "KAS", "KOS", "SOL"];

test("the table has the pipeline's header and three-character codes", () => {
  expect(header).toBe("m49,iso3");
  for (const [m49, iso] of table) {
    expect(m49).toMatch(/^\d{3}$/);
    expect(iso).toMatch(/^[A-Z]{3}$/);
  }
  expect(new Set(table.map(([m49]) => m49)).size).toBe(table.length);
});

test("every atom with a numeric code in Natural Earth has a row", () => {
  const covered = new Set(table.map(([, iso]) => iso));
  const missing = atoms.filter((iso) => !covered.has(iso)).sort();
  expect(missing).toEqual(WITHOUT_CODE);
});

test("every row points at an atom", () => {
  const known = new Set(atoms);
  for (const [, iso] of table) expect(known.has(iso)).toBe(true);
});

test("rows follow ISO 3166 numeric codes", () => {
  const code = new Map(table.map(([m49, iso]) => [iso, m49]));
  expect(code.get("USA")).toBe("840");
  expect(code.get("FRA")).toBe("250");
  expect(code.get("NOR")).toBe("578");
  expect(code.get("CHN")).toBe("156");
});
