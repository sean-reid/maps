import type { Geometry } from "./geometry.ts";

export interface Box {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

export interface Shape {
  /** Unsigned area of all the atom's rings, in grid cells. */
  area: number;
  box: Box;
  /** Centroid and box of the atom's largest ring, where its label sits. */
  anchor: { x: number; y: number; width: number; height: number; area: number };
}

/** Per-atom area, bounds and label anchor for one set of vertex positions. */
export function measure(geo: Geometry, points: Float64Array): Shape[] {
  const shapes: Shape[] = geo.atoms.map(() => ({
    area: 0,
    box: { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity },
    anchor: { x: 0, y: 0, width: 0, height: 0, area: 0 },
  }));
  const signed = new Float64Array(geo.atoms.length);
  const largest = new Float64Array(geo.atoms.length);
  for (let k = 0; k < geo.ringAtom.length; k++) {
    const shape = shapes[geo.ringAtom[k] ?? 0];
    if (!shape) continue;
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    let [a, cx, cy] = [0, 0, 0];
    const ring: Box = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
    for (let i = s, j = e - 1; i < e; j = i++) {
      const [x, y] = [points[2 * i] ?? 0, points[2 * i + 1] ?? 0];
      const [px, py] = [points[2 * j] ?? 0, points[2 * j + 1] ?? 0];
      const cross = px * y - x * py;
      a += cross;
      cx += (px + x) * cross;
      cy += (py + y) * cross;
      ring.x0 = Math.min(ring.x0, x);
      ring.x1 = Math.max(ring.x1, x);
      ring.y0 = Math.min(ring.y0, y);
      ring.y1 = Math.max(ring.y1, y);
    }
    a /= 2;
    const atom = geo.ringAtom[k] ?? 0;
    signed[atom] = (signed[atom] ?? 0) + a;
    shape.box.x0 = Math.min(shape.box.x0, ring.x0);
    shape.box.x1 = Math.max(shape.box.x1, ring.x1);
    shape.box.y0 = Math.min(shape.box.y0, ring.y0);
    shape.box.y1 = Math.max(shape.box.y1, ring.y1);
    if (Math.abs(a) > (largest[atom] ?? 0)) {
      largest[atom] = Math.abs(a);
      shape.anchor = {
        x: widestSpan(points, s, e, cy / (6 * a), cx / (6 * a)),
        y: cy / (6 * a),
        width: ring.x1 - ring.x0,
        height: ring.y1 - ring.y0,
        area: Math.abs(a),
      };
    }
  }
  shapes.forEach((shape, i) => (shape.area = Math.abs(signed[i] ?? 0)));
  return shapes;
}

/** The centroid's x if it lies inside the ring at height y, else the middle of the ring's widest span there. */
function widestSpan(points: Float64Array, s: number, e: number, y: number, x: number): number {
  const xs: number[] = [];
  for (let i = s, j = e - 1; i < e; j = i++) {
    const [xi, yi] = [points[2 * i] ?? 0, points[2 * i + 1] ?? 0];
    const [xj, yj] = [points[2 * j] ?? 0, points[2 * j + 1] ?? 0];
    if (yi > y !== yj > y) xs.push(xi + ((y - yi) / (yj - yi)) * (xj - xi));
  }
  xs.sort((a, b) => a - b);
  let best = x;
  let widest = 0;
  for (let i = 0; i + 1 < xs.length; i += 2) {
    const [a, b] = [xs[i] ?? 0, xs[i + 1] ?? 0];
    if (x >= a && x <= b) return x;
    if (b - a > widest) {
      widest = b - a;
      best = (a + b) / 2;
    }
  }
  return best;
}

export function bounds(shapes: Shape[]): Box {
  const all: Box = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
  for (const { box } of shapes) {
    all.x0 = Math.min(all.x0, box.x0);
    all.x1 = Math.max(all.x1, box.x1);
    all.y0 = Math.min(all.y0, box.y0);
    all.y1 = Math.max(all.y1, box.y1);
  }
  return all;
}

/** Index of the atom containing the grid point, or -1 over sea. */
export function atomAt(
  geo: Geometry,
  points: Float64Array,
  shapes: Shape[],
  x: number,
  y: number,
): number {
  const inside = new Uint8Array(geo.atoms.length);
  for (let k = 0; k < geo.ringAtom.length; k++) {
    const atom = geo.ringAtom[k] ?? 0;
    const box = shapes[atom]?.box;
    if (!box || x < box.x0 || x > box.x1 || y < box.y0 || y > box.y1) continue;
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    for (let i = s, j = e - 1; i < e; j = i++) {
      const [xi, yi] = [points[2 * i] ?? 0, points[2 * i + 1] ?? 0];
      const [xj, yj] = [points[2 * j] ?? 0, points[2 * j + 1] ?? 0];
      if (yi > y !== yj > y && x < xi + ((y - yi) / (yj - yi)) * (xj - xi)) {
        inside[atom] = (inside[atom] ?? 0) ^ 1;
      }
    }
  }
  // An enclave sits in its host's hole, so at most one atom is odd; prefer the smallest if not.
  let hit = -1;
  for (let i = 0; i < inside.length; i++) {
    if (inside[i] && (hit < 0 || (shapes[i]?.area ?? 0) < (shapes[hit]?.area ?? 0))) hit = i;
  }
  return hit;
}
