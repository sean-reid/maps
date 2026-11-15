import type { Camera } from "./camera.ts";
import type { Geometry } from "./geometry.ts";
import type { Shape } from "./shapes.ts";

export interface Ink {
  paper: string;
  ink: string;
  soft: string;
  font: string;
}

export const LABEL_MIN_PX = 10;
const LABEL_MAX_PX = 18;

/**
 * Label size in CSS pixels for an anchor ring of the given on-screen box and area,
 * or 0 when the name does not fit. A thin ring has a large box and little room.
 */
export function labelSize(name: string, widthPx: number, heightPx: number, areaPx: number): number {
  const room = Math.min(widthPx * heightPx, 2.5 * areaPx);
  const width = Math.min(widthPx, room / heightPx);
  const height = Math.min(heightPx, room / widthPx);
  const size = Math.min(LABEL_MAX_PX, 0.16 * Math.sqrt(room));
  const fits = 0.56 * size * name.length <= 0.7 * width && size <= 0.5 * height;
  return size >= LABEL_MIN_PX && fits ? size : 0;
}

/** Atoms drawn together as one historical entity: its internal borders and member names are left out. */
export interface Merge {
  /** Entity index per atom, or -1. */
  era: Int16Array;
  /** Per vertex, the atom across the edge to the next vertex, or -1 on coast. */
  other: Int32Array;
  /** Entity names by index. */
  names: string[];
}

export function draw(
  ctx: CanvasRenderingContext2D,
  geo: Geometry,
  points: Float64Array,
  shapes: Shape[],
  camera: Camera,
  active: number,
  missing: Uint8Array | undefined,
  merge: Merge | undefined,
  ink: Ink,
  layer: { alpha: number; clear: boolean } = { alpha: 1, clear: true },
): void {
  const { width, height } = ctx.canvas;
  // The backing store keeps the density it was sized at, whatever the window reports now.
  const dpr = width / (ctx.canvas.getBoundingClientRect().width || width);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.globalAlpha = 1;
  if (layer.clear) {
    ctx.fillStyle = ink.paper;
    ctx.fillRect(0, 0, width / dpr, height / dpr);
  }
  ctx.globalAlpha = layer.alpha;
  ctx.lineJoin = "round";
  ctx.strokeStyle = ink.ink;

  const eraOf = (atom: number): number => merge?.era[atom] ?? -1;
  const activeEra = eraOf(active);
  const all = new Path2D();
  const lifted = new Path2D();
  const empty = new Path2D();
  const ring: number[] = [];
  for (let k = 0; k < geo.ringAtom.length; k++) {
    const atom = geo.ringAtom[k] ?? 0;
    const era = eraOf(atom);
    const outline = atom === active || (era >= 0 && era === activeEra) ? lifted : all;
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    ring.length = 0;
    for (let i = s; i < e; i++) {
      const [x, y] = camera.toScreen(points[2 * i] ?? 0, points[2 * i + 1] ?? 0);
      ring.push(x, y);
    }
    const n = e - s;
    if (missing?.[atom]) {
      for (let i = 0; i < n; i++) {
        if (i === 0) empty.moveTo(ring[0] ?? 0, ring[1] ?? 0);
        else empty.lineTo(ring[2 * i] ?? 0, ring[2 * i + 1] ?? 0);
      }
      empty.closePath();
    }
    // Edge by edge, so a border between two members of one entity can be left undrawn.
    let down = false;
    for (let i = 0; i < n; i++) {
      const across = merge ? (merge.other[s + i] ?? -1) : -1;
      if (era >= 0 && across >= 0 && eraOf(across) === era) {
        down = false;
        continue;
      }
      const j = (i + 1) % n;
      if (!down) outline.moveTo(ring[2 * i] ?? 0, ring[2 * i + 1] ?? 0);
      outline.lineTo(ring[2 * j] ?? 0, ring[2 * j + 1] ?? 0);
      down = true;
    }
  }
  if (missing) {
    ctx.fillStyle = hatch(ctx, ink.soft, dpr);
    ctx.fill(empty, "evenodd");
  }
  ctx.lineWidth = 0.75;
  ctx.stroke(all);
  ctx.lineWidth = 2.5;
  ctx.stroke(lifted);

  ctx.fillStyle = ink.ink;
  ctx.strokeStyle = ink.paper;
  ctx.lineWidth = 3;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  // An entity is named once, on its largest member.
  const largest = new Map<number, number>();
  shapes.forEach((shape, i) => {
    const era = eraOf(i);
    if (era < 0) return;
    const best = largest.get(era);
    if (best === undefined || shape.anchor.area > (shapes[best]?.anchor.area ?? 0)) {
      largest.set(era, i);
    }
  });
  shapes.forEach((shape, i) => {
    const era = eraOf(i);
    if (era >= 0 && largest.get(era) !== i) return;
    const name = (era >= 0 ? merge?.names[era] : geo.atoms[i]?.name) ?? "";
    const { anchor } = shape;
    const k = camera.k;
    const size = labelSize(name, anchor.width * k, anchor.height * k, anchor.area * k * k);
    if (!size) return;
    const [x, y] = camera.toScreen(anchor.x, anchor.y);
    if (x < 0 || y < 0 || x > width / dpr || y > height / dpr) return;
    ctx.font = `500 ${size}px ${ink.font}`;
    ctx.strokeText(name, x, y);
    ctx.fillText(name, x, y);
  });
}

const patterns = new Map<string, CanvasPattern | string>();

/** Diagonal hairlines for countries the series has no value for. */
function hatch(ctx: CanvasRenderingContext2D, colour: string, dpr: number): CanvasPattern | string {
  const key = `${colour}@${dpr}`;
  let pattern = patterns.get(key);
  if (!pattern) {
    pattern = hatchPattern(ctx, colour, dpr);
    patterns.set(key, pattern);
  }
  return pattern;
}

function hatchPattern(
  ctx: CanvasRenderingContext2D,
  colour: string,
  dpr: number,
): CanvasPattern | string {
  const size = Math.round(5 * dpr);
  const tile = new OffscreenCanvas(size, size);
  const t = tile.getContext("2d");
  if (!t) return colour;
  t.strokeStyle = colour;
  t.lineWidth = Math.max(1, 0.6 * dpr);
  t.beginPath();
  t.moveTo(0, size);
  t.lineTo(size, 0);
  t.moveTo(-1, 1);
  t.lineTo(1, -1);
  t.moveTo(size - 1, size + 1);
  t.lineTo(size + 1, size - 1);
  t.stroke();
  const pattern = ctx.createPattern(tile, "repeat");
  pattern?.setTransform(new DOMMatrix().scale(1 / dpr));
  return pattern ?? colour;
}
