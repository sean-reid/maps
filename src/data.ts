import { decodeSeries } from "./series.ts";

export interface SeriesMeta {
  slug: string;
  title: string;
  unit: string;
  source: string;
  first: number;
  last: number;
  entities: number;
}

export interface Catalogue {
  version: string;
  series: SeriesMeta[];
}

export type Series = ReturnType<typeof decodeSeries>;

/** The search index, or undefined while the site has no data. */
export async function loadCatalogue(): Promise<Catalogue | undefined> {
  const res = await fetch("/api/index");
  const version = res.headers.get("x-data-version");
  if (!res.ok || !version) return undefined;
  const { series } = (await res.json()) as { series: SeriesMeta[] };
  return { version, series };
}

const cache = new Map<string, Promise<Series>>();

export function loadSeries(version: string, slug: string): Promise<Series> {
  const key = `${version}/${slug}`;
  let pending = cache.get(key);
  if (!pending) {
    pending = fetch(`/api/data/${version}/series/${slug}.bin`).then(async (res) => {
      if (!res.ok) throw new Error(`${slug} did not load (${res.status})`);
      return decodeSeries(await res.arrayBuffer());
    });
    pending.catch(() => cache.delete(key));
    cache.set(key, pending);
  }
  return pending;
}
