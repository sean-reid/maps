import type { SeriesMeta } from "./data.ts";

const words = (text: string): string[] =>
  text
    .toLowerCase()
    .normalize("NFKD")
    .replace(/\p{M}+/gu, "")
    .replace(/[^\p{L}\p{N}$%]+/gu, " ")
    .split(" ")
    .filter(Boolean);

export interface Searchable {
  meta: SeriesMeta;
  title: string[];
  rest: string[];
  /** Breadth of the series: more countries and more years rank higher among equal matches. */
  weight: number;
}

export function prepare(series: SeriesMeta[]): Searchable[] {
  return series.map((meta) => ({
    meta,
    title: words(meta.title),
    rest: words(`${meta.unit} ${meta.source}`),
    weight: Math.log2(1 + meta.entities) + Math.log2(1 + meta.last - meta.first) / 4,
  }));
}

function scoreWord(query: string, item: Searchable): number {
  let best = 0;
  item.title.forEach((word, i) => {
    const lead = i === 0 ? 1 : 0;
    if (word === query) best = Math.max(best, 8 + lead);
    else if (word.startsWith(query)) best = Math.max(best, 5 + lead);
    else if (query.length >= 3 && word.includes(query)) best = Math.max(best, 2);
  });
  if (best === 0 && item.rest.some((word) => word.startsWith(query))) best = 1;
  return best;
}

/**
 * Series matching every word of `query`, best first. A word matches a title word it equals,
 * starts, or (from three letters) sits inside, or the start of a unit or source word.
 * Order: how well the words match, then how much of the title they account for, then the
 * breadth of the series, then slug. An empty query lists the broadest series.
 */
export function search(index: Searchable[], query: string, limit = 30): SeriesMeta[] {
  const asked = words(query);
  if (asked.length === 0 && query.trim() !== "") return [];
  const scored: { item: Searchable; score: number; cover: number }[] = [];
  for (const item of index) {
    let score = 0;
    let all = true;
    for (const word of asked) {
      const s = scoreWord(word, item);
      if (s === 0) {
        all = false;
        break;
      }
      score += s;
    }
    if (!all) continue;
    scored.push({
      item,
      score: asked.length ? score / asked.length : 0,
      cover: asked.length ? Math.min(1, asked.length / item.title.length) : 0,
    });
  }
  scored.sort(
    (a, b) =>
      b.score - a.score ||
      b.cover - a.cover ||
      b.item.weight - a.item.weight ||
      a.item.meta.slug.localeCompare(b.item.meta.slug),
  );
  return scored.slice(0, limit).map((s) => s.item.meta);
}
