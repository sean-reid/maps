export interface Feel {
  /** Natural frequency in radians per second. */
  omega: number;
  /** Damping ratio: below 1 overshoots, 1 settles without. */
  zeta: number;
  /** Longest wait before a vertex starts, in seconds. */
  stagger: number;
}

/** One visible overshoot, settled in about 900 ms. */
export const BOUNCE: Feel = { omega: 9, zeta: 0.5, stagger: 0.12 };
/** No overshoot, for year-by-year playback where the map never rests. */
export const FOLLOW: Feel = { omega: 20, zeta: 1, stagger: 0 };

/** Grid cells from the goal, and cells per second, under which a vertex counts as at rest. */
const REST = 0.01;
const REST_SPEED = 0.2;
const MAX_STEP = 1 / 120;

/** Vertices that began at the same place, so borders two countries share move as one. */
export function groupVertices(points: Float64Array): Uint32Array {
  const ids = new Map<string, number>();
  const group = new Uint32Array(points.length / 2);
  for (let i = 0; i < group.length; i++) {
    const key = `${points[2 * i]},${points[2 * i + 1]}`;
    let id = ids.get(key);
    if (id === undefined) {
      id = ids.size;
      ids.set(key, id);
    }
    group[i] = id;
  }
  return group;
}

/** A damped spring per vertex pulling the drawn map towards the solved one. */
export class Springs {
  readonly position: Float64Array;
  private velocity: Float64Array;
  private from: Float64Array;
  private goal: Float64Array;
  private delay: Float32Array;
  private clock = 0;
  private feel: Feel = BOUNCE;

  constructor(start: Float64Array) {
    this.position = start.slice();
    this.velocity = new Float64Array(start.length);
    this.from = start.slice();
    this.goal = start.slice();
    this.delay = new Float32Array(start.length / 2);
  }

  /**
   * Aims at `goal`. `lead` ranks vertices from 0 (moves first) to 1 (moves last);
   * a vertex waits `lead * feel.stagger` seconds before it lets go of its old goal.
   */
  aim(goal: Float64Array, feel: Feel, lead?: Float32Array): void {
    for (let i = 0; i < this.delay.length; i++) {
      const held = this.clock < (this.delay[i] ?? 0);
      if (!held) {
        this.from[2 * i] = this.goal[2 * i] ?? 0;
        this.from[2 * i + 1] = this.goal[2 * i + 1] ?? 0;
      }
      this.delay[i] = lead ? (lead[i] ?? 0) * feel.stagger : 0;
    }
    this.goal.set(goal);
    this.feel = feel;
    this.clock = 0;
  }

  /** Puts every vertex on its goal at rest. */
  snap(): void {
    this.position.set(this.goal);
    this.from.set(this.goal);
    this.velocity.fill(0);
    this.delay.fill(0);
  }

  /** Advances by `seconds`; returns whether anything is still moving. */
  step(seconds: number): boolean {
    if (!(seconds > 0)) return true;
    let moving = false;
    for (let left = Math.min(seconds, 0.1); left > 1e-9; left -= MAX_STEP) {
      moving = this.advance(Math.min(left, MAX_STEP));
    }
    return moving;
  }

  private advance(dt: number): boolean {
    const { omega, zeta } = this.feel;
    const { position, velocity, from, goal, delay } = this;
    this.clock += dt;
    let moving = false;
    for (let i = 0; i < position.length; i++) {
      const waiting = this.clock < (delay[i >> 1] ?? 0);
      const target = (waiting ? from[i] : goal[i]) ?? 0;
      const x = position[i] ?? 0;
      const v =
        (velocity[i] ?? 0) +
        (-omega * omega * (x - target) - 2 * zeta * omega * (velocity[i] ?? 0)) * dt;
      velocity[i] = v;
      position[i] = x + v * dt;
      if (waiting || Math.abs(v) > REST_SPEED || Math.abs(x + v * dt - target) > REST)
        moving = true;
    }
    if (!moving) this.snap();
    return moving;
  }
}

/**
 * Stagger order: vertices of the atoms whose area changes most go first.
 * `ratio[a]` is an atom's new area over its old; `owner[i]` is the atom of vertex i.
 */
export function leadByChange(
  ratio: Float64Array,
  owner: Uint32Array,
  group: Uint32Array,
): Float32Array {
  const change = Float64Array.from(ratio, (r) =>
    r > 0 && Number.isFinite(r) ? Math.abs(Math.log(r)) : 0,
  );
  const most = Math.max(...change, 1e-9);
  const groups = new Float32Array(Math.max(...group, 0) + 1).fill(1);
  for (let i = 0; i < owner.length; i++) {
    const g = group[i] ?? 0;
    groups[g] = Math.min(groups[g] ?? 1, 1 - (change[owner[i] ?? 0] ?? 0) / most);
  }
  return Float32Array.from(owner, (_, i) => groups[group[i] ?? 0] ?? 1);
}
