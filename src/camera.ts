import type { Box } from "./shapes.ts";

export const MAX_ZOOM = 16;

/** Maps grid cells to CSS pixels: a fit of the map into the viewport, then a user zoom and pan. */
export class Camera {
  zoom = 1;
  private panX = 0;
  private panY = 0;
  private scale = 1;
  private originX = 0;
  private originY = 0;
  private width = 1;
  private height = 1;

  fit(map: Box, width: number, height: number, margin = 8): void {
    this.panX *= width / this.width;
    this.panY *= height / this.height;
    this.width = width;
    this.height = height;
    const [w, h] = [map.x1 - map.x0, map.y1 - map.y0];
    this.scale = Math.min((width - 2 * margin) / w, (height - 2 * margin) / h);
    this.originX = (width - this.scale * w) / 2 - this.scale * map.x0;
    // In a tall viewport the map sits under its title, not adrift in the middle.
    const top = Math.min((height - this.scale * h) / 2, margin + 0.08 * height);
    this.originY = top - this.scale * map.y0;
    this.clamp();
  }

  /** Pixels per grid cell. */
  get k(): number {
    return this.scale * this.zoom;
  }

  toScreen(x: number, y: number): [number, number] {
    return [
      (this.originX + this.scale * x) * this.zoom + this.panX,
      (this.originY + this.scale * y) * this.zoom + this.panY,
    ];
  }

  toGrid(sx: number, sy: number): [number, number] {
    return [
      ((sx - this.panX) / this.zoom - this.originX) / this.scale,
      ((sy - this.panY) / this.zoom - this.originY) / this.scale,
    ];
  }

  /** Multiplies the zoom by `factor`, keeping the point under (sx, sy) still. */
  zoomAt(sx: number, sy: number, factor: number): void {
    const next = Math.min(MAX_ZOOM, Math.max(1, this.zoom * factor));
    const ratio = next / this.zoom;
    this.panX = sx - (sx - this.panX) * ratio;
    this.panY = sy - (sy - this.panY) * ratio;
    this.zoom = next;
    this.clamp();
  }

  panBy(dx: number, dy: number): void {
    this.panX += dx;
    this.panY += dy;
    this.clamp();
  }

  reset(): void {
    this.zoom = 1;
    this.clamp();
  }

  private clamp(): void {
    this.panX = Math.min(0, Math.max(this.width * (1 - this.zoom), this.panX));
    this.panY = Math.min(0, Math.max(this.height * (1 - this.zoom), this.panY));
  }
}
