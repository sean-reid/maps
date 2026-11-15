import type { Geometry } from "../geometry.ts";

interface Exports {
  memory: WebAssembly.Memory;
  alloc_f64(len: number): number;
  free_f64(ptr: number, len: number): void;
  error_ptr(): number;
  error_len(): number;
  solver_new(
    lx: number,
    ly: number,
    regions: number,
    rings: number,
    ringStart: number,
    ringRegion: number,
    points: number,
    pts: number,
  ): number;
  solver_solve(
    solver: number,
    values: number,
    regions: number,
    start: number,
    skipBlur: number,
    out: number,
  ): number;
  solver_mean_error(solver: number): number;
  solver_truncated(solver: number): number;
}

export interface Solution {
  points: Float64Array;
  /** Area-weighted relative error against the target areas. */
  meanError: number;
  truncated: boolean;
}

export interface Solver {
  /**
   * One value per atom, NaN for missing. Every solve starts from the undeformed map: continuing
   * from an earlier solution compounds its folds, and a year would then look different depending
   * on how it was reached.
   */
  solve(values: Float64Array): Solution;
}

export async function createSolver(wasm: BufferSource, geo: Geometry): Promise<Solver> {
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const x = instance.exports as unknown as Exports;
  const regions = geo.atoms.length;
  const rings = geo.ringAtom.length;
  const count = geo.points.length / 2;

  const failure = (): Error => {
    const bytes = new Uint8Array(x.memory.buffer, x.error_ptr(), x.error_len());
    return new Error(new TextDecoder().decode(bytes.slice()));
  };
  // Views are made per use: growing the memory detaches earlier ones.
  const f64 = (ptr: number, len: number) => new Float64Array(x.memory.buffer, ptr, len);
  const u32 = (ptr: number, len: number) => new Uint32Array(x.memory.buffer, ptr, len);

  const startPtr = x.alloc_f64(Math.ceil((rings + 1) / 2));
  const ownerPtr = x.alloc_f64(Math.ceil(rings / 2));
  const ptsPtr = x.alloc_f64(2 * count);
  u32(startPtr, rings + 1).set(geo.ringStart);
  u32(ownerPtr, rings).set(geo.ringAtom);
  f64(ptsPtr, 2 * count).set(geo.points);
  const solver = x.solver_new(
    geo.width,
    geo.height,
    regions,
    rings,
    startPtr,
    ownerPtr,
    count,
    ptsPtr,
  );
  x.free_f64(startPtr, Math.ceil((rings + 1) / 2));
  x.free_f64(ownerPtr, Math.ceil(rings / 2));
  x.free_f64(ptsPtr, 2 * count);
  if (!solver) throw failure();
  const valuesPtr = x.alloc_f64(regions);
  const outPtr = x.alloc_f64(2 * count);

  return {
    solve(values) {
      if (values.length !== regions)
        throw new Error(`${values.length} values for ${regions} atoms`);
      f64(valuesPtr, regions).set(values);
      const status = x.solver_solve(solver, valuesPtr, regions, 0, 0, outPtr);
      if (status !== 0) throw failure();
      return {
        points: f64(outPtr, 2 * count).slice(),
        meanError: x.solver_mean_error(solver),
        truncated: x.solver_truncated(solver) !== 0,
      };
    },
  };
}
