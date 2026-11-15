import { loadCatalogue, loadSeries, type SeriesMeta } from "./data.ts";
import { layout, type Topology } from "./geometry.ts";
import { createMap } from "./map.ts";
import { SolverClient } from "./solver/client.ts";
import { BOUNCE, type Feel, FOLLOW } from "./spring.ts";
import { latestFullYear, rowsFor, valuesAt } from "./values.ts";

const OPENING = "wdi-sp-pop-totl";
/** Shortest time a year stays up during playback. */
const YEAR_MS = 350;

function need<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) throw new Error(`page is missing ${selector}`);
  return el;
}

const canvas = need<HTMLCanvasElement>("#map");
const readout = need<HTMLElement>("#readout");
const title = need<HTMLElement>("#title");
const detail = need<HTMLElement>("#detail");
const timeBar = need<HTMLElement>("#time");
const playButton = need<HTMLButtonElement>("#play");
const yearInput = need<HTMLInputElement>("#year");
const yearLabel = need<HTMLElement>("#year-label");

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
  if (!res.headers.get("content-type")?.includes("json")) throw new Error("the map did not load");
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
  if (!catalogue) return;
  // Assigned once playback exists, so a solver failure can stop it.
  let halt = (): void => undefined;
  const solver = new SolverClient(geo, (message) => {
    console.error(message);
    halt();
    readout.textContent = "The map stopped responding. Reload the page to carry on.";
  });
  const asked = parsePath(location.pathname);
  const linked = catalogue.series.find((s: SeriesMeta) => s.slug === asked.slug);
  const meta = linked ?? catalogue.series.find((s: SeriesMeta) => s.slug === OPENING);
  if (!meta) return;
  const series = await loadSeries(catalogue.version, meta.slug);
  const rows = rowsFor(series, geo.atoms);
  const [first, last] = [series.firstYear, series.firstYear + series.years - 1];
  const wanted = (linked && asked.year) || latestFullYear(series);
  let year = Math.min(last, Math.max(first, wanted));
  let playing = false;

  title.textContent = meta.title;
  detail.textContent = [meta.unit, meta.source].filter(Boolean).join(", ");
  yearInput.min = String(first);
  yearInput.max = String(last);
  timeBar.hidden = last === first;

  /** Solves and shows `to`; resolves once the map has started towards it, or at once if overtaken. */
  const showYear = async (to: number, feel: Feel): Promise<void> => {
    year = to;
    yearInput.value = String(to);
    yearLabel.textContent = String(to);
    const values = valuesAt(series, rows, to);
    const points = await solver.solve(values, shown.frame !== undefined);
    if (!points) return;
    shown.frame = {
      values,
      total: values.reduce((sum, v) => sum + (Number.isFinite(v) ? v : 0), 0),
      unit: meta.unit,
      whole: "of the world",
    };
    map.setMissing(Uint8Array.from(values, (v) => (Number.isFinite(v) ? 0 : 1)));
    const root = document.documentElement;
    root.dataset.series = meta.slug;
    root.dataset.year = String(to);
    root.dataset.rest = "false";
    writePath(`/${meta.slug}/${to}`);
    void map.moveTo(points, feel, feel === BOUNCE).then(() => {
      if (root.dataset.year === String(to)) root.dataset.rest = "true";
    });
  };

  // Each run of playback holds a number; stopping or restarting retires it, so an old loop cannot resume.
  let run = 0;
  const stop = (): void => {
    run++;
    playing = false;
    playButton.textContent = "Play";
  };

  const play = async (): Promise<void> => {
    const mine = ++run;
    playing = true;
    playButton.textContent = "Pause";
    if (year >= last) await showYear(first, FOLLOW);
    while (mine === run && year < last) {
      const tick = new Promise((done) => setTimeout(done, YEAR_MS));
      await showYear(year + 1, FOLLOW);
      await tick;
    }
    if (mine === run) stop();
  };

  halt = stop;

  playButton.addEventListener("click", () => {
    if (playing) stop();
    else void play();
  });
  yearInput.addEventListener("input", () => {
    stop();
    void showYear(Number(yearInput.value), FOLLOW);
  });

  await showYear(year, BOUNCE);
}

let pathTimer: ReturnType<typeof setTimeout> | undefined;

/** Puts `path` in the address bar once changes settle; browsers cap how often the history may be written. */
function writePath(path: string): void {
  clearTimeout(pathTimer);
  pathTimer = setTimeout(() => {
    try {
      history.replaceState(null, "", path);
    } catch {
      // The cap was hit; the next settled change writes the path.
    }
  }, 300);
}

/** Reads `/<slug>` or `/<slug>/<year>`. */
function parsePath(path: string): { slug: string | undefined; year: number | undefined } {
  const match = /^\/([a-z0-9]+(?:-[a-z0-9]+)*)(?:\/(\d{4}))?\/?$/.exec(path);
  return { slug: match?.[1], year: match?.[2] ? Number(match[2]) : undefined };
}

start()
  .catch((error: unknown) => {
    readout.textContent = error instanceof Error ? error.message : "The map did not load.";
  })
  .finally(() => {
    document.documentElement.dataset.ready = "true";
  });
