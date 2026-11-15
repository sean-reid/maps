import { loadCatalogue, loadSeries, type Series, type SeriesMeta } from "./data.ts";
import { layout, type Topology } from "./geometry.ts";
import { createMap } from "./map.ts";
import { createSearchBox } from "./searchbox.ts";
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
const searchInput = need<HTMLInputElement>("#search");
const searchList = need<HTMLElement>("#results");

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

  /** With nothing to size countries by, the plain map is the series: land area. */
  const showLand = (): void => {
    title.textContent = "Land area";
    detail.textContent = "share of land outside Antarctica, Natural Earth";
    timeBar.hidden = true;
    canvas.removeAttribute("data-waiting");
  };
  /** A failure before the first map is on screen leaves the plain one to look at. */
  const landIfWaiting = (): void => {
    if (canvas.hasAttribute("data-waiting")) showLand();
  };

  const catalogue = await loadCatalogue().catch(() => undefined);
  if (!catalogue) return showLand();
  // Assigned once playback exists, so a solver failure can stop it.
  let halt = (): void => undefined;
  const solver = new SolverClient(geo, (message) => {
    console.error(message);
    halt();
    landIfWaiting();
    readout.textContent = "The map stopped responding. Reload the page to carry on.";
  });
  const fallback = catalogue.series.find((s: SeriesMeta) => s.slug === OPENING);
  if (!fallback) return showLand();

  /** The series on the map and where its years run. */
  interface Open {
    meta: SeriesMeta;
    series: Series;
    rows: Int32Array;
    first: number;
    last: number;
  }
  // Undefined while a series loads, so the slider and playback cannot act on the one being left.
  let open: Open | undefined;
  let target: SeriesMeta | undefined;
  let year = 0;
  let playing = false;
  // Retired by anything that starts a different series, year or playback, so stale work stops.
  let run = 0;

  /** Solves and shows `to`; resolves once the map has started towards it, or at once if overtaken. */
  const showYear = async (to: number, feel: Feel): Promise<void> => {
    if (!open) {
      yearInput.value = String(year);
      return;
    }
    const { meta, series, rows } = open;
    year = to;
    yearInput.value = String(to);
    yearLabel.textContent = String(to);
    const values = valuesAt(series, rows, to);
    const points = await solver.solve(values);
    if (!points || open?.meta !== meta) return;
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
    // The first map appears already in shape; only a change from one map to another is motion.
    if (canvas.hasAttribute("data-waiting")) {
      map.jumpTo(points);
      canvas.removeAttribute("data-waiting");
      root.dataset.rest = "true";
      return;
    }
    void map.moveTo(points, feel, feel === BOUNCE).then(() => {
      if (root.dataset.series === meta.slug && root.dataset.year === String(to)) {
        root.dataset.rest = "true";
      }
    });
  };

  const stop = (): void => {
    run++;
    playing = false;
    playButton.textContent = "Play";
  };

  /** Puts a series on the map at `wanted`, or at its last well-covered year. */
  const openSeries = async (meta: SeriesMeta, wanted: number | undefined): Promise<void> => {
    stop();
    const mine = run;
    // `year` outlives a load in flight, so a second pick still carries the year on screen.
    const shownYear = year || undefined;
    open = undefined;
    target = meta;
    let series: Series;
    try {
      series = await loadSeries(catalogue.version, meta.slug);
    } catch (error) {
      if (mine !== run) return;
      target = undefined;
      throw error;
    }
    if (mine !== run) return;
    const [first, last] = [series.firstYear, series.firstYear + series.years - 1];
    open = { meta, series, rows: rowsFor(series, geo.atoms), first, last };
    title.textContent = meta.title;
    detail.textContent = [meta.unit, meta.source].filter(Boolean).join(", ");
    document.title = `${meta.title} · maps`;
    yearInput.min = String(first);
    yearInput.max = String(last);
    timeBar.hidden = last === first;
    const kept = shownYear !== undefined && shownYear >= first && shownYear <= last;
    const to = wanted ?? (kept ? shownYear : latestFullYear(series));
    await showYear(Math.min(last, Math.max(first, to)), BOUNCE);
  };

  const report = (error: unknown): void => {
    landIfWaiting();
    readout.textContent = error instanceof Error ? error.message : "That series did not load.";
  };

  const openPath = (path: string): Promise<void> => {
    const asked = parsePath(path);
    const linked = catalogue.series.find((s: SeriesMeta) => s.slug === asked.slug);
    return openSeries(linked ?? fallback, linked ? asked.year : undefined);
  };

  const play = async (): Promise<void> => {
    if (!open) return;
    const { first, last } = open;
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

  createSearchBox(searchInput, searchList, catalogue.series, (meta) => {
    canvas.focus({ preventScroll: true });
    if (meta === target) return;
    // A new series is a new history entry; it opens at the year on screen where it has that year.
    flushPath();
    history.pushState(null, "", `/${meta.slug}`);
    openSeries(meta, undefined).catch(report);
  });
  window.addEventListener("popstate", () => {
    dropPath();
    openPath(location.pathname).catch(report);
  });

  await openPath(location.pathname).catch(report);
}

let pathTimer: ReturnType<typeof setTimeout> | undefined;
let pendingPath: string | undefined;

/** Writes the waiting path now, so a history entry about to be pushed lands after it. */
function flushPath(): void {
  clearTimeout(pathTimer);
  if (pendingPath === undefined) return;
  try {
    history.replaceState(null, "", pendingPath);
  } catch {
    // Browsers cap how often the history may be written; the next settled change writes the path.
  }
  pendingPath = undefined;
}

/** Forgets the waiting path: the entry it was meant for is no longer the current one. */
function dropPath(): void {
  clearTimeout(pathTimer);
  pendingPath = undefined;
}

/** Puts `path` in the address bar once changes settle. */
function writePath(path: string): void {
  clearTimeout(pathTimer);
  pendingPath = path;
  pathTimer = setTimeout(flushPath, 300);
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
