import { Camera } from "./camera.ts";
import { draw, type Ink } from "./draw.ts";
import { layout, type Topology } from "./geometry.ts";
import { atomAt, bounds, measure } from "./shapes.ts";

const TAP_SLOP_PX = 6;

function need<T extends Element>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) throw new Error(`page is missing ${selector}`);
  return el;
}

const canvas = need<HTMLCanvasElement>("#map");
const readout = need<HTMLElement>("#readout");

async function start(): Promise<void> {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("this browser has no 2d canvas");
  const res = await fetch("/geo/atoms.json");
  if (!res.ok) throw new Error(`the map did not load (${res.status})`);
  const geo = layout((await res.json()) as Topology);
  const points = geo.points;
  const shapes = measure(geo, points);
  const land = shapes.reduce((sum, s) => sum + s.area, 0);
  const camera = new Camera();
  let active = -1;
  let pinned = false;
  let announced = false;
  let queued = false;

  const ink = (): Ink => {
    const css = getComputedStyle(document.documentElement);
    return {
      paper: css.getPropertyValue("--paper").trim(),
      ink: css.getPropertyValue("--ink").trim(),
      font: css.getPropertyValue("--sans").trim(),
    };
  };

  const paint = (): void => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      draw(ctx, geo, points, shapes, camera, active, ink());
    });
  };

  const resize = (): void => {
    const dpr = window.devicePixelRatio || 1;
    const { width, height } = canvas.getBoundingClientRect();
    if (width <= 0 || height <= 0) return;
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    camera.fit(bounds(shapes), width, height);
    paint();
  };

  const share = (value: number): string => {
    const pct = (100 * value) / land;
    if (pct < 0.01) return "<0.01%";
    return `${pct < 0.1 ? pct.toFixed(2) : pct.toFixed(1)}%`;
  };

  const show = (atom: number, announce: boolean): void => {
    if (atom === active && announce === announced) return;
    active = atom;
    announced = announce;
    const shape = shapes[atom];
    readout.setAttribute("aria-live", announce ? "polite" : "off");
    readout.replaceChildren();
    if (shape) {
      const name = document.createElement("b");
      name.textContent = geo.atoms[atom]?.name ?? "";
      const num = document.createElement("span");
      num.className = "num";
      num.textContent = share(shape.area);
      const rest = document.createElement("span");
      rest.className = "soft";
      rest.textContent = "of land outside Antarctica";
      readout.append(name, " ", num, " ", rest);
    }
    paint();
  };

  const pick = (atom: number): void => {
    pinned = atom >= 0 && !(pinned && atom === active);
    show(pinned ? atom : -1, true);
  };

  const local = (event: { clientX: number; clientY: number }): [number, number] => {
    const box = canvas.getBoundingClientRect();
    return [event.clientX - box.left, event.clientY - box.top];
  };

  const atomUnder = (sx: number, sy: number): number => {
    const [x, y] = camera.toGrid(sx, sy);
    return atomAt(geo, points, shapes, x, y);
  };

  const touches = new Map<number, [number, number]>();
  let travelled = 0;
  let wasPinch = false;

  canvas.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    canvas.setPointerCapture(event.pointerId);
    if (touches.size === 0) {
      travelled = 0;
      wasPinch = false;
    } else {
      wasPinch = true;
    }
    touches.set(event.pointerId, local(event));
  });

  canvas.addEventListener("pointermove", (event) => {
    const before = touches.get(event.pointerId);
    const now = local(event);
    if (!before) {
      if (event.pointerType === "mouse" && !pinned) show(atomUnder(...now), false);
      return;
    }
    const other = [...touches.entries()].find(([id]) => id !== event.pointerId)?.[1];
    if (other) {
      const was = Math.hypot(before[0] - other[0], before[1] - other[1]);
      const is = Math.hypot(now[0] - other[0], now[1] - other[1]);
      if (was > 0) camera.zoomAt((now[0] + other[0]) / 2, (now[1] + other[1]) / 2, is / was);
      touches.set(event.pointerId, now);
    } else {
      travelled += Math.hypot(now[0] - before[0], now[1] - before[1]);
      if (travelled < TAP_SLOP_PX) return;
      camera.panBy(now[0] - before[0], now[1] - before[1]);
      touches.set(event.pointerId, now);
    }
    paint();
  });

  const release = (event: PointerEvent): void => {
    if (!touches.delete(event.pointerId)) return;
    const tap = event.type === "pointerup" && touches.size === 0 && !wasPinch;
    if (tap && travelled < TAP_SLOP_PX) pick(atomUnder(...local(event)));
  };
  canvas.addEventListener("pointerup", release);
  canvas.addEventListener("pointercancel", release);
  canvas.addEventListener("lostpointercapture", release);
  canvas.addEventListener("pointerleave", () => {
    if (!pinned && touches.size === 0) show(-1, false);
  });

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const pixels = event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 400 : 1);
      camera.zoomAt(...local(event), Math.exp(-pixels * 0.0015));
      paint();
    },
    { passive: false },
  );

  canvas.addEventListener("dblclick", () => {
    camera.reset();
    paint();
  });

  canvas.addEventListener("keydown", (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
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
      Enter: () => pick(atomUnder(width / 2, height / 2)),
      Escape: () => pick(-1),
    };
    const act = keys[event.key];
    if (!act) return;
    event.preventDefault();
    act();
    paint();
  });

  const watchDensity = (): void => {
    const query = matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
    query.addEventListener(
      "change",
      () => {
        resize();
        watchDensity();
      },
      { once: true },
    );
  };
  watchDensity();
  new ResizeObserver(resize).observe(canvas);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
  void document.fonts.ready.then(paint);
  resize();
}

start()
  .catch((error: unknown) => {
    readout.textContent = error instanceof Error ? error.message : "The map did not load.";
  })
  .finally(() => {
    document.documentElement.dataset.ready = "true";
  });
