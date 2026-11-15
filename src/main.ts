import { loadCatalogue, loadSeries, type SeriesMeta } from "./data.ts";
import { layout, type Topology } from "./geometry.ts";
import { createMap } from "./map.ts";
import { SolverClient } from "./solver/client.ts";
import { latestFullYear, rowsFor, valuesAt } from "./values.ts";

const OPENING = "wdi-sp-pop-totl";

function need<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) throw new Error(`page is missing ${selector}`);
  return el;
}

const canvas = need<HTMLCanvasElement>("#map");
const readout = need<HTMLElement>("#readout");
const title = need<HTMLElement>("#title");
const detail = need<HTMLElement>("#detail");

/** What the map currently shows: a value per atom and how to word it. */
interface Frame {
  values: Float64Array;
  total: number;
  unit: string;
  whole: string;
}

const percent = (part: number, whole: number): string => {
  const pct = (100 * part) / whole;
  if (pct < 0.01) return "<0.01%";
  return `${pct < 0.1 ? pct.toFixed(2) : pct.toFixed(1)}%`;
};

const amount = new Intl.NumberFormat("en", { maximumFractionDigits: 0 });

async function start(): Promise<void> {
  const res = await fetch("/geo/atoms.json");
  if (!res.ok) throw new Error(`the map did not load (${res.status})`);
  const geo = layout((await res.json()) as Topology);
  const shown: { frame?: Frame } = {};

  const map = createMap(canvas, geo, (atom, announce) => {
    readout.setAttribute("aria-live", announce ? "polite" : "off");
    readout.replaceChildren();
    const { frame } = shown;
    const value = frame ? frame.values[atom] : map.shapes()[atom]?.area;
    if (atom < 0 || value === undefined) return;
    const name = document.createElement("b");
    name.textContent = geo.atoms[atom]?.name ?? "";
    readout.append(name);
    const soft = (text: string): HTMLElement => {
      const el = document.createElement("span");
      el.className = "soft";
      el.textContent = text;
      return el;
    };
    const num = (text: string): HTMLElement => {
      const el = document.createElement("span");
      el.className = "num";
      el.textContent = text;
      return el;
    };
    if (!Number.isFinite(value)) {
      readout.append(" ", soft("no data"));
    } else if (frame) {
      readout.append(" ", num(amount.format(value)), " ");
      if (frame.unit) readout.append(soft(frame.unit), " ");
      readout.append(num(percent(value, frame.total)), " ", soft(frame.whole));
    } else {
      const land = map.shapes().reduce((sum, s) => sum + s.area, 0);
      readout.append(" ", num(percent(value, land)), " ", soft("of land outside Antarctica"));
    }
  });
  document.documentElement.dataset.ready = "true";

  const catalogue = await loadCatalogue();
  const meta = catalogue?.series.find((s: SeriesMeta) => s.slug === OPENING);
  if (!catalogue || !meta) return;
  const solver = new SolverClient(geo, (message) => console.error(message));
  const series = await loadSeries(catalogue.version, meta.slug);
  const rows = rowsFor(series, geo.atoms);
  const year = latestFullYear(series);
  const values = valuesAt(series, rows, year);
  const points = await solver.solve(values, false);
  if (!points) return;
  shown.frame = {
    values,
    total: values.reduce((sum, v) => sum + (Number.isFinite(v) ? v : 0), 0),
    unit: meta.unit,
    whole: "of the world",
  };
  title.textContent = meta.title;
  detail.textContent = [meta.unit, String(year), meta.source].filter(Boolean).join(", ");
  map.setMissing(Uint8Array.from(values, (v) => (Number.isFinite(v) ? 0 : 1)));
  map.setPoints(points);
  document.documentElement.dataset.series = meta.slug;
}

start()
  .catch((error: unknown) => {
    readout.textContent = error instanceof Error ? error.message : "The map did not load.";
  })
  .finally(() => {
    document.documentElement.dataset.ready = "true";
  });
