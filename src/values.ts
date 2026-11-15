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

export interface Span {
  /** First and last year worth showing: at least half as many countries as the best year. */
  first: number;
  last: number;
  /** The last year with at least 80% of the best year's countries, where the map opens. */
  opening: number;
}

/**
 * The years of a series that have enough countries to make a map. A country counts for every
 * year between its first and last value, since the gaps between are interpolated when shown.
 */
export function coveredYears(series: Series): Span {
  const counts = new Array<number>(series.years).fill(0);
  for (let row = 0; row < series.codes.length; row++) {
    let [from, to] = [-1, -1];
    for (let y = 0; y < series.years; y++) {
      if (!Number.isFinite(series.values[row * series.years + y])) continue;
      if (from < 0) from = y;
      to = y;
    }
    for (let y = from; y >= 0 && y <= to; y++) counts[y] = (counts[y] ?? 0) + 1;
  }
  const most = Math.max(0, ...counts);
  const enough = (share: number): number[] =>
    counts.flatMap((n, y) => (most > 0 && n >= share * most ? [series.firstYear + y] : []));
  const shown = enough(0.5);
  const first = shown[0] ?? series.firstYear;
  const last = shown.at(-1) ?? series.firstYear;
  return { first, last, opening: enough(0.8).at(-1) ?? last };
}
