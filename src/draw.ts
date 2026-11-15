import type { Camera } from "./camera.ts";
import type { Geometry } from "./geometry.ts";
import type { Shape } from "./shapes.ts";

export interface Ink {
  paper: string;
  ink: string;
  font: string;
}

export const LABEL_MIN_PX = 10;
const LABEL_MAX_PX = 18;

/** Label size in CSS pixels for an anchor of the given on-screen size, or 0 when the name does not fit. */
export function labelSize(name: string, widthPx: number, heightPx: number): number {
  const size = Math.min(LABEL_MAX_PX, 0.16 * Math.sqrt(widthPx * heightPx));
  const fits = 0.56 * size * name.length <= 0.7 * widthPx && size <= 0.5 * heightPx;
  return size >= LABEL_MIN_PX && fits ? size : 0;
}

export function draw(
  ctx: CanvasRenderingContext2D,
  geo: Geometry,
  points: Float64Array,
  shapes: Shape[],
  camera: Camera,
  active: number,
  ink: Ink,
): void {
  const { width, height } = ctx.canvas;
  // The backing store keeps the density it was sized at, whatever the window reports now.
  const dpr = width / (ctx.canvas.getBoundingClientRect().width || width);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = ink.paper;
  ctx.fillRect(0, 0, width / dpr, height / dpr);
  ctx.lineJoin = "round";
  ctx.strokeStyle = ink.ink;

  const all = new Path2D();
  const lifted = new Path2D();
  for (let k = 0; k < geo.ringAtom.length; k++) {
    const path = geo.ringAtom[k] === active ? lifted : all;
    const [s, e] = [geo.ringStart[k] ?? 0, geo.ringStart[k + 1] ?? 0];
    for (let i = s; i < e; i++) {
      const [x, y] = camera.toScreen(points[2 * i] ?? 0, points[2 * i + 1] ?? 0);
      if (i === s) path.moveTo(x, y);
      else path.lineTo(x, y);
    }
    path.closePath();
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
  shapes.forEach((shape, i) => {
    const name = geo.atoms[i]?.name ?? "";
    const { anchor } = shape;
    const size = labelSize(name, anchor.width * camera.k, anchor.height * camera.k);
    if (!size) return;
    const [x, y] = camera.toScreen(anchor.x, anchor.y);
    if (x < 0 || y < 0 || x > width / dpr || y > height / dpr) return;
    ctx.font = `500 ${size}px ${ink.font}`;
    ctx.strokeText(name, x, y);
    ctx.fillText(name, x, y);
  });
}
