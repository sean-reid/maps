import { expect, test } from "vitest";
import { BOUNCE, FOLLOW, groupVertices, leadByChange, Springs } from "../../src/spring.ts";

function run(springs: Springs, seconds: number, watch?: (t: number) => void): number {
  let t = 0;
  let moving = true;
  while (moving && t < seconds) {
    moving = springs.step(1 / 60);
    t += 1 / 60;
    watch?.(t);
  }
  return t;
}

test("the bounce overshoots once, visibly, and rests on the goal within a second or so", () => {
  const springs = new Springs(Float64Array.of(0, 0));
  springs.aim(Float64Array.of(100, 0), BOUNCE);
  let peak = 0;
  let dip = 100;
  let lastVisible = 0;
  const settled = run(springs, 5, (t) => {
    const x = springs.position[0] ?? 0;
    if (Math.abs(x - 100) > 2) lastVisible = t;
    if (x > peak) {
      peak = x;
      dip = x;
    }
    dip = Math.min(dip, x);
  });
  expect(peak).toBeGreaterThan(110);
  expect(peak).toBeLessThan(125);
  expect(dip).toBeGreaterThan(95);
  expect(lastVisible).toBeGreaterThan(0.5);
  expect(lastVisible).toBeLessThan(1);
  expect(settled).toBeLessThan(2.5);
  expect(springs.position[0]).toBe(100);
});

test("following does not overshoot", () => {
  const springs = new Springs(Float64Array.of(0, 0));
  springs.aim(Float64Array.of(100, 0), FOLLOW);
  let peak = 0;
  const settled = run(springs, 5, () => (peak = Math.max(peak, springs.position[0] ?? 0)));
  expect(peak).toBeLessThanOrEqual(100.01);
  expect(settled).toBeLessThan(1);
});

test("a vertex with a later lead holds still, then follows", () => {
  const springs = new Springs(Float64Array.of(0, 0, 0, 0));
  springs.aim(Float64Array.of(10, 0, 10, 0), BOUNCE, Float32Array.of(0, 1));
  springs.step(BOUNCE.stagger / 2);
  expect(springs.position[0]).toBeGreaterThan(0);
  expect(springs.position[2]).toBe(0);
  run(springs, 5);
  expect([...springs.position]).toEqual([10, 0, 10, 0]);
});

test("a new aim mid-flight keeps the motion continuous", () => {
  const springs = new Springs(Float64Array.of(0, 0));
  springs.aim(Float64Array.of(100, 0), BOUNCE);
  springs.step(0.1);
  const before = springs.position[0] ?? 0;
  springs.aim(Float64Array.of(-50, 0), BOUNCE);
  springs.step(1 / 120);
  expect(Math.abs((springs.position[0] ?? 0) - before)).toBeLessThan(5);
  run(springs, 5);
  expect(springs.position[0]).toBe(-50);
});

test("a frame with no time in it leaves the spring in flight", () => {
  const springs = new Springs(Float64Array.of(0, 0));
  springs.aim(Float64Array.of(100, 0), BOUNCE);
  expect(springs.step(0)).toBe(true);
  expect(springs.step(-0.004)).toBe(true);
  expect(springs.position[0]).toBe(0);
});

test("a huge frame gap does not blow the spring up", () => {
  const springs = new Springs(Float64Array.of(0, 0));
  springs.aim(Float64Array.of(100, 0), BOUNCE);
  springs.step(30);
  expect(Math.abs(springs.position[0] ?? 0)).toBeLessThan(200);
});

test("shared border vertices get one lead, set by the neighbour that changes most", () => {
  // Two atoms; vertex 1 of atom 0 and vertex 2 of atom 1 start at the same place.
  const points = Float64Array.of(0, 0, 5, 5, 5, 5, 9, 9);
  const group = groupVertices(points);
  expect([...group]).toEqual([0, 1, 1, 2]);
  const lead = leadByChange(Float64Array.of(1.1, 4), Uint32Array.of(0, 0, 1, 1), group);
  expect(lead[1]).toBe(lead[2]);
  expect(lead[3]).toBe(0);
  expect(lead[0]).toBeGreaterThan(0.9);
});
