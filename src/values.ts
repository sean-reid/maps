import type { Series } from "./data.ts";
import type { Atom } from "./geometry.ts";

/** Codes a source uses for a country the map knows under another. */
const ALIAS: Record<string, string> = { XKX: "KOS" };

/** Row of each atom in the series, or -1 when the series has no such country. */
export function rowsFor(series: Series, atoms: Atom[]): Int32Array {
  const row = new Map(series.codes.map((code, i) => [ALIAS[code] ?? code, i]));
  return Int32Array.from(atoms, (atom) => row.get(atom.iso) ?? -1);
}

/** One value per atom for `year`, linearly interpolated across gaps; NaN outside a country's known span. */
export function valuesAt(series: Series, rows: Int32Array, year: number): Float64Array {
  const out = new Float64Array(rows.length).fill(NaN);
  const y = year - series.firstYear;
  if (y < 0 || y >= series.years) return out;
  rows.forEach((row, atom) => {
    if (row < 0) return;
    const at = (i: number) => series.values[row * series.years + i] ?? NaN;
    if (Number.isFinite(at(y))) {
      out[atom] = at(y);
      return;
    }
    let [before, after] = [y - 1, y + 1];
    while (before >= 0 && !Number.isFinite(at(before))) before--;
    while (after < series.years && !Number.isFinite(at(after))) after++;
    if (before < 0 || after >= series.years) return;
    out[atom] = at(before) + ((at(after) - at(before)) * (y - before)) / (after - before);
  });
  return out;
}

/** The last year in which most of the series' countries report, so the opening map is not half empty. */
export function latestFullYear(series: Series): number {
  const counts = Array.from({ length: series.years }, (_, y) => {
    let n = 0;
    for (let row = 0; row < series.codes.length; row++) {
      if (Number.isFinite(series.values[row * series.years + y])) n++;
    }
    return n;
  });
  const most = Math.max(...counts);
  for (let y = series.years - 1; y >= 0; y--) {
    if ((counts[y] ?? 0) >= 0.8 * most) return series.firstYear + y;
  }
  return series.firstYear;
}
