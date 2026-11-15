//! Flow-based contiguous cartogram: every vertex is carried by the flow that
//! evens out a density field of target area over current area, so regions
//! reach their target areas while shared borders stay shared.

use rustdct::{DctPlanner, TransformType2And3};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Options {
    pub max_iter: usize,
    /// Stop once every region is within this relative area error.
    pub tol: f64,
    /// Smallest target as a fraction of total area.
    pub min_frac: f64,
    /// Smallest target as a fraction of the region's own original area; thinner regions fold.
    pub min_ratio: f64,
    /// Target for a region with no value, as a fraction of its own original area.
    pub missing_frac: f64,
    /// Gaussian blur of the first iteration in cells; halves each iteration.
    pub blur0: f64,
    /// Largest vertex displacement per advection step in cells.
    pub max_step_cells: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_iter: 8,
            tol: 0.01,
            min_frac: 2e-5,
            min_ratio: 0.05,
            missing_frac: 0.08,
            blur0: 4.0,
            max_step_cells: 0.75,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub iters: usize,
    pub steps: usize,
    /// Largest relative area error over regions.
    pub max_err: f64,
    /// Relative area error weighted by target area.
    pub mean_err: f64,
    /// An advection hit its step cap before the flow finished.
    pub truncated: bool,
}

const MAX_STEPS: usize = 2000;
/// How far one pass may ask a region to grow or shrink. A region smaller than a cell can be
/// thousands of times short of its target, and one such cell would set the pace for every vertex.
const MAX_DENSITY: f64 = 30.0;

struct Flow<'a> {
    rho: &'a [f64],
    jx: &'a [f64],
    jy: &'a [f64],
    mean: f64,
}

pub struct Cartogram {
    lx: usize,
    ly: usize,
    n_regions: usize,
    ring_start: Vec<usize>,
    ring_region: Vec<usize>,
    pts: Vec<f64>,
    dct_x: Arc<dyn TransformType2And3<f64>>,
    dct_y: Arc<dyn TransformType2And3<f64>>,
}

impl Cartogram {
    /// `ring_start` holds point offsets with a trailing sentinel; `pts` is x,y pairs in grid units.
    pub fn new(
        lx: usize,
        ly: usize,
        n_regions: usize,
        ring_start: Vec<usize>,
        ring_region: Vec<usize>,
        pts: Vec<f64>,
    ) -> Result<Self, String> {
        if lx < 2 || ly < 2 {
            return Err(format!("grid {lx}x{ly} is smaller than 2x2"));
        }
        if ring_start.len() != ring_region.len() + 1 || ring_start[0] != 0 {
            return Err(
                "ring_start needs one offset per ring plus a sentinel, starting at 0".into(),
            );
        }
        if ring_start.windows(2).any(|w| w[1] < w[0] + 3) {
            return Err("every ring needs at least 3 points".into());
        }
        if pts.len() != 2 * ring_start[ring_start.len() - 1] {
            return Err("point count does not match the ring offsets".into());
        }
        if ring_region.iter().any(|&r| r >= n_regions) {
            return Err("ring region index out of range".into());
        }
        if pts.iter().any(|v| !v.is_finite()) {
            return Err("points must be finite".into());
        }
        let mut planner = DctPlanner::new();
        Ok(Cartogram {
            lx,
            ly,
            n_regions,
            ring_start,
            ring_region,
            pts,
            dct_x: planner.plan_dct2(lx),
            dct_y: planner.plan_dct2(ly),
        })
    }

    pub fn points(&self) -> &[f64] {
        &self.pts
    }

    pub fn areas(&self, pts: &[f64]) -> Vec<f64> {
        let mut a = vec![0.0; self.n_regions];
        for (k, &r) in self.ring_region.iter().enumerate() {
            a[r] += ring_area(&pts[2 * self.ring_start[k]..2 * self.ring_start[k + 1]]);
        }
        for v in a.iter_mut() {
            *v = v.abs();
        }
        a
    }

    /// Target areas for `values` (NaN = missing), preserving total area.
    /// Floored and missing regions hold their floor, regions without area get none,
    /// and the rest share what is left by value.
    pub fn targets(&self, values: &[f64], opt: &Options) -> Vec<f64> {
        assert_eq!(values.len(), self.n_regions, "one value per region");
        let a0 = self.areas(&self.pts);
        let total: f64 = a0.iter().sum();
        if total <= 0.0 {
            return a0;
        }
        let floor = |i: usize| (opt.min_ratio * a0[i]).max(opt.min_frac * total);
        let mut fixed: Vec<Option<f64>> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                if a0[i] <= 0.0 {
                    Some(0.0)
                } else if v.is_finite() {
                    None
                } else {
                    Some(opt.missing_frac * a0[i])
                }
            })
            .collect();
        loop {
            let held: f64 = fixed.iter().flatten().sum();
            let vsum: f64 = (0..values.len())
                .filter(|&i| fixed[i].is_none())
                .map(|i| values[i].max(0.0))
                .sum();
            let share = |i: usize| {
                if vsum > 0.0 {
                    values[i].max(0.0) / vsum * (total - held)
                } else {
                    0.0
                }
            };
            let below: Vec<usize> = (0..values.len())
                .filter(|&i| fixed[i].is_none() && share(i) < floor(i))
                .collect();
            if below.is_empty() {
                let t: Vec<f64> = (0..values.len())
                    .map(|i| fixed[i].unwrap_or_else(|| share(i)))
                    .collect();
                let s = total / t.iter().sum::<f64>();
                return t.into_iter().map(|x| x * s).collect();
            }
            for i in below {
                fixed[i] = Some(floor(i));
            }
        }
    }

    pub fn solve(&self, values: &[f64], opt: &Options) -> Result<(Vec<f64>, Stats), String> {
        self.solve_from(values, opt, &self.pts, 0)
    }

    /// Continues from `start` (a previous solution), skipping the first `skip_blur` blur levels.
    pub fn solve_from(
        &self,
        values: &[f64],
        opt: &Options,
        start: &[f64],
        skip_blur: usize,
    ) -> Result<(Vec<f64>, Stats), String> {
        if values.len() != self.n_regions {
            return Err(format!(
                "{} values for {} regions",
                values.len(),
                self.n_regions
            ));
        }
        if start.len() != self.pts.len() || start.iter().any(|v| !v.is_finite()) {
            return Err("start must be finite and the size of the geometry".into());
        }
        let target = self.targets(values, opt);
        let mut pts = start.to_vec();
        let n = self.lx * self.ly;
        let mut rho = vec![0.0; n];
        let mut phi = vec![0.0; n];
        let mut jx = vec![0.0; n];
        let mut jy = vec![0.0; n];
        let mut stats = Stats {
            iters: 0,
            steps: 0,
            max_err: f64::MAX,
            mean_err: f64::MAX,
            truncated: false,
        };
        for it in 0..opt.max_iter {
            let area = self.areas(&pts);
            let (mx, mean) = errors(&area, &target);
            stats.max_err = mx;
            stats.mean_err = mean;
            if mx < opt.tol {
                break;
            }
            let dens: Vec<f64> = target
                .iter()
                .zip(&area)
                .map(|(t, a)| (t / a.max(1e-9)).clamp(1.0 / MAX_DENSITY, MAX_DENSITY))
                .collect();
            self.rasterize(&pts, &dens, &mut rho);
            let sigma = opt.blur0 * 0.5f64.powi((it + skip_blur).min(64) as i32);
            let mean_rho = self.poisson(&mut rho, &mut phi, if sigma > 0.2 { sigma } else { 0.0 });
            self.gradient(&phi, &mut jx, &mut jy);
            let flow = Flow {
                rho: &rho,
                jx: &jx,
                jy: &jy,
                mean: mean_rho,
            };
            let (steps, finished) = self.advect(&mut pts, &flow, opt.max_step_cells);
            stats.steps += steps;
            stats.truncated |= !finished;
            stats.iters = it + 1;
        }
        let area = self.areas(&pts);
        let (mx, mean) = errors(&area, &target);
        stats.max_err = mx;
        stats.mean_err = mean;
        Ok((pts, stats))
    }

    fn rasterize(&self, pts: &[f64], dens: &[f64], rho: &mut [f64]) {
        rho.iter_mut().for_each(|v| *v = 1.0);
        let mut region_rings: Vec<Vec<usize>> = vec![Vec::new(); self.n_regions];
        for (k, &r) in self.ring_region.iter().enumerate() {
            region_rings[r].push(k);
        }
        let mut xs: Vec<f64> = Vec::new();
        for (r, rings) in region_rings.iter().enumerate() {
            let (mut y0, mut y1) = (f64::MAX, f64::MIN);
            for &k in rings {
                for i in self.ring_start[k]..self.ring_start[k + 1] {
                    y0 = y0.min(pts[2 * i + 1]);
                    y1 = y1.max(pts[2 * i + 1]);
                }
            }
            if y0 > y1 {
                continue;
            }
            let ra = ((y0 - 0.5).ceil().max(0.0)) as usize;
            let rb = ((y1 - 0.5).floor().min(self.ly as f64 - 1.0)).max(0.0) as usize;
            for row in ra..=rb.min(self.ly - 1) {
                let yc = row as f64 + 0.5;
                xs.clear();
                for &k in rings {
                    let (s, e) = (self.ring_start[k], self.ring_start[k + 1]);
                    let mut j = e - 1;
                    for i in s..e {
                        let (xa, ya, xb, yb) =
                            (pts[2 * j], pts[2 * j + 1], pts[2 * i], pts[2 * i + 1]);
                        if (ya <= yc) != (yb <= yc) {
                            xs.push(xa + (yc - ya) / (yb - ya) * (xb - xa));
                        }
                        j = i;
                    }
                }
                xs.sort_by(f64::total_cmp);
                for pair in xs.as_chunks::<2>().0 {
                    let ca = ((pair[0] - 0.5).ceil().max(0.0)) as usize;
                    let cb = (pair[1] - 0.5).floor();
                    if cb < 0.0 {
                        continue;
                    }
                    let cb = (cb as usize).min(self.lx - 1);
                    for c in ca..=cb {
                        rho[row * self.lx + c] = dens[r];
                    }
                }
            }
        }
    }

    /// Blurs `rho` in place, solves laplacian(phi) = rho - mean with Neumann walls, returns the mean.
    fn poisson(&self, rho: &mut [f64], phi: &mut [f64], sigma: f64) -> f64 {
        let (lx, ly) = (self.lx, self.ly);
        let mut hat = rho.to_vec();
        self.dct2d(&mut hat, true);
        let norm = 4.0 / (lx * ly) as f64;
        let mean = hat[0] * norm / 4.0;
        let mut blurred = hat.clone();
        for j in 0..ly {
            let ky = std::f64::consts::PI * j as f64 / ly as f64;
            let ey = 2.0 * ky.cos() - 2.0;
            for i in 0..lx {
                let kx = std::f64::consts::PI * i as f64 / lx as f64;
                let ex = 2.0 * kx.cos() - 2.0;
                let g = (-0.5 * sigma * sigma * (kx * kx + ky * ky)).exp();
                let k = j * lx + i;
                let w = norm * if i == 0 { 0.5 } else { 1.0 } * if j == 0 { 0.5 } else { 1.0 };
                blurred[k] = hat[k] * g * w;
                hat[k] = if i == 0 && j == 0 {
                    0.0
                } else {
                    hat[k] * g * w / (ex + ey)
                };
            }
        }
        self.dct2d(&mut blurred, false);
        self.dct2d(&mut hat, false);
        rho.copy_from_slice(&blurred);
        phi.copy_from_slice(&hat);
        mean
    }

    fn dct2d(&self, a: &mut [f64], forward: bool) {
        let (lx, ly) = (self.lx, self.ly);
        for row in a.chunks_exact_mut(lx) {
            if forward {
                self.dct_x.process_dct2(row);
            } else {
                row[0] *= 2.0;
                self.dct_x.process_dct3(row);
            }
        }
        let mut col = vec![0.0; ly];
        for i in 0..lx {
            for j in 0..ly {
                col[j] = a[j * lx + i];
            }
            if forward {
                self.dct_y.process_dct2(&mut col);
            } else {
                col[0] *= 2.0;
                self.dct_y.process_dct3(&mut col);
            }
            for j in 0..ly {
                a[j * lx + i] = col[j];
            }
        }
    }

    fn gradient(&self, phi: &[f64], jx: &mut [f64], jy: &mut [f64]) {
        let (lx, ly) = (self.lx, self.ly);
        for j in 0..ly {
            for i in 0..lx {
                let k = j * lx + i;
                let l = if i > 0 { phi[k - 1] } else { phi[k] };
                let r = if i + 1 < lx { phi[k + 1] } else { phi[k] };
                let u = if j > 0 { phi[k - lx] } else { phi[k] };
                let d = if j + 1 < ly { phi[k + lx] } else { phi[k] };
                jx[k] = 0.5 * (r - l);
                jy[k] = 0.5 * (d - u);
            }
        }
    }

    fn sample(&self, f: &[f64], x: f64, y: f64) -> f64 {
        let gx = (x - 0.5).clamp(0.0, self.lx as f64 - 1.0);
        let gy = (y - 0.5).clamp(0.0, self.ly as f64 - 1.0);
        let i = (gx as usize).min(self.lx - 2);
        let j = (gy as usize).min(self.ly - 2);
        let (fx, fy) = (gx - i as f64, gy - j as f64);
        let k = j * self.lx + i;
        (1.0 - fy) * ((1.0 - fx) * f[k] + fx * f[k + 1])
            + fy * ((1.0 - fx) * f[k + self.lx] + fx * f[k + self.lx + 1])
    }

    fn velocity(&self, f: &Flow, t: f64, x: f64, y: f64) -> (f64, f64) {
        let d = ((1.0 - t) * self.sample(f.rho, x, y) + t * f.mean).max(1e-9);
        (self.sample(f.jx, x, y) / d, self.sample(f.jy, x, y) / d)
    }

    /// Returns the step count and whether the flow ran to its end.
    fn advect(&self, pts: &mut [f64], f: &Flow, max_step: f64) -> (usize, bool) {
        let mut t = 0.0;
        let mut steps = 0;
        let np = pts.len() / 2;
        let mut vel = vec![0.0; pts.len()];
        while t < 1.0 && steps < MAX_STEPS {
            let mut vmax: f64 = 1e-12;
            for p in 0..np {
                let (vx, vy) = self.velocity(f, t, pts[2 * p], pts[2 * p + 1]);
                vel[2 * p] = vx;
                vel[2 * p + 1] = vy;
                vmax = vmax.max(vx.abs()).max(vy.abs());
            }
            let dt = (max_step / vmax).min(1.0 - t);
            for p in 0..np {
                let (x, y) = (pts[2 * p], pts[2 * p + 1]);
                let (mx, my) = (x + 0.5 * dt * vel[2 * p], y + 0.5 * dt * vel[2 * p + 1]);
                let (vx, vy) = self.velocity(f, t + 0.5 * dt, mx, my);
                pts[2 * p] = (x + dt * vx).clamp(0.0, self.lx as f64);
                pts[2 * p + 1] = (y + dt * vy).clamp(0.0, self.ly as f64);
            }
            t += dt;
            steps += 1;
        }
        (steps, t >= 1.0)
    }
}

pub fn ring_area(p: &[f64]) -> f64 {
    let n = p.len() / 2;
    let mut a = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        a += p[2 * j] * p[2 * i + 1] - p[2 * i] * p[2 * j + 1];
        j = i;
    }
    0.5 * a
}

fn errors(area: &[f64], target: &[f64]) -> (f64, f64) {
    let total: f64 = target.iter().sum();
    let mut mx: f64 = 0.0;
    let mut mean = 0.0;
    for (a, t) in area.iter().zip(target) {
        // A region with no area has no vertices the flow could move apart.
        if *a <= 0.0 {
            continue;
        }
        let e = (a - t).abs() / t;
        mx = mx.max(e);
        mean += e * t / total;
    }
    (mx, mean)
}

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(test)]
mod tests;
