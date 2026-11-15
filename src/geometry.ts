export interface Topology {
  transform: { scale: [number, number]; translate: [number, number] };
  arcs: number[][][];
  objects: { atoms: { geometries: AtomGeometry[] } };
}

interface AtomGeometry {
  type: "Polygon" | "MultiPolygon";
  arcs: number[][] | number[][][];
  properties: { iso: string; name: string };
}

export interface Atom {
  iso: string;
  name: string;
}

export interface Geometry {
  width: number;
  height: number;
  atoms: Atom[];
  /** Point offset of each ring, with a trailing sentinel. */
  ringStart: Uint32Array;
  ringAtom: Uint32Array;
  /** x,y pairs in grid cells, y down. A ring does not repeat its first point. */
  points: Float64Array;
}

export const GRID = { width: 512, height: 256, pad: 0.15, maxEdge: 1 };

/** Lays the topology out on the solver grid, with every arc split into edges of at most one cell. */
export function layout(topology: Topology, grid = GRID): Geometry {
  const { scale, translate } = topology.transform;
  let [x0, x1, y0, y1] = [Infinity, -Infinity, Infinity, -Infinity];
  const raw = topology.arcs.map((arc) => {
    let [x, y] = [0, 0];
    return arc.map(([dx = 0, dy = 0]) => {
      x += dx;
      y += dy;
      const p: [number, number] = [x * scale[0] + translate[0], -(y * scale[1] + translate[1])];
      x0 = Math.min(x0, p[0]);
      x1 = Math.max(x1, p[0]);
      y0 = Math.min(y0, p[1]);
      y1 = Math.max(y1, p[1]);
      return p;
    });
  });
  const inner = 1 - 2 * grid.pad;
  const k = Math.min((grid.width * inner) / (x1 - x0), (grid.height * inner) / (y1 - y0));
  const ox = (grid.width - k * (x1 - x0)) / 2;
  const oy = (grid.height - k * (y1 - y0)) / 2;

  const arcs = raw.map((arc) => {
    const out: number[] = [];
    arc.forEach(([px, py], i) => {
      const x = ox + k * (px - x0);
      const y = oy + k * (py - y0);
      if (i > 0) {
        const ax = out[out.length - 2] ?? x;
        const ay = out[out.length - 1] ?? y;
        if (x === ax && y === ay) return;
        const steps = Math.ceil(Math.hypot(x - ax, y - ay) / grid.maxEdge);
        for (let s = 1; s < steps; s++) {
          out.push(ax + ((x - ax) * s) / steps, ay + ((y - ay) * s) / steps);
        }
      }
      out.push(x, y);
    });
    return out;
  });

  const atoms: Atom[] = [];
  const ringStart: number[] = [0];
  const ringAtom: number[] = [];
  const points: number[] = [];
  topology.objects.atoms.geometries.forEach((g, index) => {
    atoms.push(g.properties);
    const polygons = (g.type === "Polygon" ? [g.arcs] : g.arcs) as number[][][];
    for (const ring of polygons.flat()) {
      for (const ref of ring) {
        const arc = arcs[ref < 0 ? ~ref : ref];
        if (!arc) throw new Error(`ring refers to missing arc ${ref}`);
        const n = arc.length / 2;
        // The last point of an arc is the first point of the next.
        for (let i = 0; i < n - 1; i++) {
          const j = ref < 0 ? n - 1 - i : i;
          points.push(arc[2 * j] ?? 0, arc[2 * j + 1] ?? 0);
        }
      }
      ringStart.push(points.length / 2);
      ringAtom.push(index);
    }
  });
  return {
    width: grid.width,
    height: grid.height,
    atoms,
    ringStart: Uint32Array.from(ringStart),
    ringAtom: Uint32Array.from(ringAtom),
    points: Float64Array.from(points),
  };
}
