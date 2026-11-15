import { Camera } from "./camera.ts";
import { draw, type Ink } from "./draw.ts";
import { layout, type Topology } from "./geometry.ts";
import { atomAt, bounds, measure } from "./shapes.ts";

const canvas = document.querySelector<HTMLCanvasElement>("#map");
const readout = document.querySelector<HTMLElement>("#readout");
if (!canvas || !readout) throw new Error("page is missing the map");
const ctx = canvas.getContext("2d");
if (!ctx) throw new Error("no 2d canvas");

const res = await fetch("/geo/atoms.json");
if (!res.ok) throw new Error(`map data: ${res.status}`);
const geo = layout((await res.json()) as Topology);
const points = geo.points;
const shapes = measure(geo, points);
const land = shapes.reduce((sum, s) => sum + s.area, 0);
const camera = new Camera();
let active = -1;
let pinned = false;
let queued = false;

function ink(): Ink {
  const css = getComputedStyle(document.documentElement);
  return {
    paper: css.getPropertyValue("--paper").trim(),
    ink: css.getPropertyValue("--ink").trim(),
    font: css.getPropertyValue("--sans").trim(),
  };
}

function paint(): void {
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    draw(ctx!, geo, points, shapes, camera, active, ink(), window.devicePixelRatio || 1);
  });
}

function resize(): void {
  const dpr = window.devicePixelRatio || 1;
  const { width, height } = canvas!.getBoundingClientRect();
  canvas!.width = Math.round(width * dpr);
  canvas!.height = Math.round(height * dpr);
  camera.fit(bounds(shapes), width, height);
  paint();
}

function share(value: number): string {
  const pct = (100 * value) / land;
  return `${pct < 0.1 ? pct.toFixed(2) : pct.toFixed(1)}%`;
}

function show(atom: number): void {
  if (atom === active) return;
  active = atom;
  const shape = shapes[atom];
  readout!.replaceChildren();
  if (shape) {
    const name = document.createElement("b");
    name.textContent = geo.atoms[atom]?.name ?? "";
    const num = document.createElement("span");
    num.className = "num";
    num.textContent = share(shape.area);
    const rest = document.createElement("span");
    rest.className = "soft";
    rest.textContent = "of land outside Antarctica";
    readout!.append(name, num, rest);
  }
  paint();
}

function local(event: { clientX: number; clientY: number }): [number, number] {
  const box = canvas!.getBoundingClientRect();
  return [event.clientX - box.left, event.clientY - box.top];
}

function atomUnder(event: { clientX: number; clientY: number }): number {
  const [x, y] = camera.toGrid(...local(event));
  return atomAt(geo, points, shapes, x, y);
}

const touches = new Map<number, [number, number]>();
let moved = 0;

canvas.addEventListener("pointerdown", (event) => {
  canvas.setPointerCapture(event.pointerId);
  touches.set(event.pointerId, local(event));
  if (touches.size === 1) moved = 0;
});

canvas.addEventListener("pointermove", (event) => {
  const before = touches.get(event.pointerId);
  const now = local(event);
  if (!before) {
    if (event.pointerType === "mouse" && !pinned) show(atomUnder(event));
    return;
  }
  const others = [...touches.entries()].filter(([id]) => id !== event.pointerId);
  const other = others[0]?.[1];
  if (other) {
    const was = Math.hypot(before[0] - other[0], before[1] - other[1]);
    const is = Math.hypot(now[0] - other[0], now[1] - other[1]);
    if (was > 0) camera.zoomAt((now[0] + other[0]) / 2, (now[1] + other[1]) / 2, is / was);
  } else {
    camera.panBy(now[0] - before[0], now[1] - before[1]);
  }
  moved += Math.hypot(now[0] - before[0], now[1] - before[1]);
  touches.set(event.pointerId, now);
  paint();
});

function release(event: PointerEvent): void {
  if (!touches.delete(event.pointerId)) return;
  if (event.type === "pointerup" && touches.size === 0 && moved < 6) {
    const atom = atomUnder(event);
    pinned = atom >= 0 && !(pinned && atom === active);
    show(pinned ? atom : event.pointerType === "mouse" ? atom : -1);
  }
}
canvas.addEventListener("pointerup", release);
canvas.addEventListener("pointercancel", release);
canvas.addEventListener("pointerleave", () => {
  if (!pinned && touches.size === 0) show(-1);
});

canvas.addEventListener(
  "wheel",
  (event) => {
    event.preventDefault();
    camera.zoomAt(...local(event), Math.exp(-event.deltaY * 0.0015));
    paint();
  },
  { passive: false },
);

canvas.addEventListener("dblclick", () => {
  camera.reset();
  paint();
});

canvas.addEventListener("keydown", (event) => {
  const { width, height } = canvas.getBoundingClientRect();
  const step = 48;
  const keys: Record<string, () => void> = {
    "+": () => camera.zoomAt(width / 2, height / 2, 1.5),
    "=": () => camera.zoomAt(width / 2, height / 2, 1.5),
    "-": () => camera.zoomAt(width / 2, height / 2, 1 / 1.5),
    "0": () => camera.reset(),
    ArrowLeft: () => camera.panBy(step, 0),
    ArrowRight: () => camera.panBy(-step, 0),
    ArrowUp: () => camera.panBy(0, step),
    ArrowDown: () => camera.panBy(0, -step),
  };
  const act = keys[event.key];
  if (!act) return;
  event.preventDefault();
  act();
  paint();
});

new ResizeObserver(resize).observe(canvas);
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
await document.fonts.ready;
resize();
document.documentElement.dataset.ready = "true";
