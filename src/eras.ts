import type { Series } from "./data.ts";
import type { Atom, Geometry } from "./geometry.ts";
import { groupVertices } from "./spring.ts";

/** A reporting unit that no longer exists, made of countries the map draws today. */
export interface Era {
  /** The code sources' rows carry for it; never a current ISO code. */
  code: string;
  name: string;
  members: string[];
}

export const ERAS: Era[] = [
  {
    code: "SUN",
    name: "USSR",
    members: [
      "ARM",
      "AZE",
      "BLR",
      "EST",
      "GEO",
      "KAZ",
      "KGZ",
      "LVA",
      "LTU",
      "MDA",
      "RUS",
      "TJK",
      "TKM",
      "UKR",
      "UZB",
    ],
  },
  { code: "YUG", name: "Yugoslavia", members: ["BIH", "HRV", "KOS", "MKD", "MNE", "SRB", "SVN"] },
  { code: "SCG", name: "Serbia and Montenegro", members: ["KOS", "MNE", "SRB"] },
  { code: "CSK", name: "Czechoslovakia", members: ["CZE", "SVK"] },
  { code: "BLX", name: "Belgium-Luxembourg", members: ["BEL", "LUX"] },
  { code: "SDF", name: "Sudan (former)", members: ["SDN", "SSD"] },
  { code: "ETF", name: "Ethiopia (former)", members: ["ERI", "ETH"] },
  { code: "ANT", name: "Netherlands Antilles", members: ["CUW", "SXM"] },
  { code: "PCI", name: "Pacific Islands Trust Territory", members: ["FSM", "MHL", "MNP", "PLW"] },
];

export interface EraValues {
  /** One value per atom, with each active era's figure spread over its members by land area. */
  values: Float64Array;
  /** Index into `ERAS` of the era an atom is drawn as part of this year, or -1. */
  era: Int16Array;
  /** The figure reported for each active era, by index into `ERAS`. */
  totals: Map<number, number>;
}

/** A row's value for a year index, interpolated across its gaps, NaN outside its span. */
function at(series: Series, row: number, y: number): number {
  const cell = (i: number) => series.values[row * series.years + i] ?? NaN;
  if (y < 0 || y >= series.years) return NaN;
  if (Number.isFinite(cell(y))) return cell(y);
  let [before, after] = [y - 1, y + 1];
  while (before >= 0 && !Number.isFinite(cell(before))) before--;
  while (after < series.years && !Number.isFinite(cell(after))) after++;
  if (before < 0 || after >= series.years) return NaN;
  return cell(before) + ((cell(after) - cell(before)) * (y - before)) / (after - before);
}

/**
 * Folds historical entities into per-atom values. An era is active in a year when the series
 * has a figure for it and for none of its members; its members then share the figure by
 * `land` (plain map area), so the solver moves them as one country.
 */
export function withEras(
  series: Series,
  atoms: Atom[],
  land: ArrayLike<number>,
  year: number,
  values: Float64Array,
): EraValues {
  const out = values.slice();
  const era = new Int16Array(atoms.length).fill(-1);
  const totals = new Map<number, number>();
  const index = new Map(atoms.map((atom, i) => [atom.iso, i]));
  const y = year - series.firstYear;
  ERAS.forEach((entity, e) => {
    const row = series.codes.indexOf(entity.code);
    if (row < 0) return;
    const total = at(series, row, y);
    const members = entity.members.flatMap((iso) => index.get(iso) ?? []);
    if (!Number.isFinite(total) || members.length === 0) return;
    if (members.some((m) => Number.isFinite(values[m]) || (era[m] ?? -1) >= 0)) return;
    const area = members.reduce((sum, m) => sum + (land[m] ?? 0), 0);
    if (!(area > 0)) return;
    for (const m of members) {
      out[m] = (total * (land[m] ?? 0)) / area;
      era[m] = e;
    }
    totals.set(e, total);
  });
  return { values: out, era, totals };
}

/**
 * For every vertex i, the atom on the other side of the edge from i to the next vertex of its
 * ring, or -1 where the edge is coast. Two rings share an edge when both ends began at the same
 * places.
 */
export function neighbours(geo: Geometry): Int32Array {
  const group = groupVertices(geo.points);
  const owners = new Map<number, number[]>();
  const key = (a: number, b: number): number => Math.min(a, b) * group.length + Math.max(a, b);
  const edges: number[] = new Array<number>(group.length).fill(-1);
  geo.ringAtom.forEach((atom, k) => {
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    for (let i = s; i < e; i++) {
      const j = i + 1 < e ? i + 1 : s;
      const edge = key(group[i] ?? 0, group[j] ?? 0);
      edges[i] = edge;
      const list = owners.get(edge) ?? [];
      list.push(atom);
      owners.set(edge, list);
    }
  });
  const other = new Int32Array(group.length).fill(-1);
  geo.ringAtom.forEach((atom, k) => {
    for (let i = geo.ringStart[k] ?? 0; i < (geo.ringStart[k + 1] ?? 0); i++) {
      const across = (owners.get(edges[i] ?? -1) ?? []).find((a) => a !== atom);
      if (across !== undefined) other[i] = across;
    }
  });
  return other;
}
