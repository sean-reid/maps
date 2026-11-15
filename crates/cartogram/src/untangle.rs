use super::{ring_area, Cartogram};

/// Loops that folding leaves in thin regions are short; a longer reach collapses the large side of a loop.
const LOOKAHEAD: usize = 40;
/// A ring smaller than this has no area worth keeping.
const MIN_AREA: f64 = 1e-9;
/// A collapse can expose a crossing with a neighbouring edge; later passes rarely find more.
const PASSES: usize = 4;

/// Vertices that start at one position, as runs of `members` indexed by `start`.
pub(crate) struct Groups {
    of: Vec<usize>,
    ring: Vec<usize>,
    start: Vec<usize>,
    members: Vec<usize>,
}

impl Groups {
    pub(crate) fn new(pts: &[f64], ring_start: &[usize]) -> Self {
        let n = pts.len() / 2;
        let mut ring = vec![0; n];
        for (k, w) in ring_start.windows(2).enumerate() {
            ring[w[0]..w[1]].fill(k);
        }
        let key = |i: usize| (pts[2 * i].to_bits(), pts[2 * i + 1].to_bits());
        let mut members: Vec<usize> = (0..n).collect();
        members.sort_by_key(|&i| (key(i), i));
        let mut of = vec![0; n];
        let mut start = vec![0];
        for (k, &i) in members.iter().enumerate() {
            if k > 0 && key(members[k - 1]) != key(i) {
                start.push(k);
            }
            of[i] = start.len() - 1;
        }
        start.push(n);
        Groups {
            of,
            ring,
            start,
            members,
        }
    }

    fn with(&self, v: usize) -> &[usize] {
        let g = self.of[v];
        &self.members[self.start[g]..self.start[g + 1]]
    }
}

fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// The parameter along (c, d) where it crosses (a, b), if the two properly cross.
fn crossing(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> Option<f64> {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    (o1 * o2 < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0).then(|| o1 / (o1 - o2))
}

struct Scratch {
    saved: Vec<(usize, f64, f64)>,
    rings: Vec<usize>,
}

impl Cartogram {
    /// Pulls each loop of a self-crossing outline onto its crossing point, moving vertices that
    /// began together as one; the vertex groups come from the original points, so a start that
    /// does not keep shared borders shared is the caller's to fix.
    /// Returns how many vertices ended up somewhere else and how many crossings within reach remain.
    pub(crate) fn untangle(&self, pts: &mut [f64]) -> (usize, usize) {
        let before = pts.to_vec();
        let rings = self.ring_region.len();
        let area0: Vec<f64> = (0..rings).map(|k| self.ring_abs_area(pts, k)).collect();
        let mut scratch = Scratch {
            saved: Vec::new(),
            rings: Vec::new(),
        };
        for _ in 0..PASSES {
            let mut changed = false;
            for k in 0..rings {
                let (s, e) = (self.ring_start[k], self.ring_start[k + 1]);
                changed |= self.untangle_ring(pts, s, e, &area0, &mut scratch);
            }
            if !changed {
                break;
            }
        }
        let moved = (0..pts.len() / 2)
            .filter(|&i| pts[2 * i..2 * i + 2] != before[2 * i..2 * i + 2])
            .count();
        let left = (0..rings)
            .map(|k| count_ring(pts, self.ring_start[k], self.ring_start[k + 1]))
            .sum();
        (moved, left)
    }

    fn ring_abs_area(&self, pts: &[f64], k: usize) -> f64 {
        ring_area(&pts[2 * self.ring_start[k]..2 * self.ring_start[k + 1]]).abs()
    }

    fn untangle_ring(
        &self,
        pts: &mut [f64],
        s: usize,
        e: usize,
        area0: &[f64],
        scratch: &mut Scratch,
    ) -> bool {
        let n = e - s;
        let reach = LOOKAHEAD.min(n - 2);
        let mut changed = false;
        let mut i = 0;
        while i < n {
            let (a, b) = (at(pts, s, n, i), at(pts, s, n, i + 1));
            let mut hit = None;
            for d in 2..=reach {
                let (c, dd) = (at(pts, s, n, i + d), at(pts, s, n, i + d + 1));
                if let Some(t) = crossing(a, b, c, dd) {
                    hit = Some((d, (c.0 + t * (dd.0 - c.0), c.1 + t * (dd.1 - c.1))));
                    break;
                }
            }
            if let Some((d, x)) = hit {
                let ahead = d <= n - d;
                let (from, count) = if ahead {
                    (i + 1, d)
                } else {
                    (i + d + 1, n - d)
                };
                if self.collapse(pts, (s, n), (from, count), x, area0, scratch) {
                    changed = true;
                    if ahead {
                        i += d;
                    }
                }
            }
            i += 1;
        }
        changed
    }

    /// Moves `count` vertices of the ring from offset `from` onto `x`, undoing it if a ring that
    /// had area would be left with none.
    fn collapse(
        &self,
        pts: &mut [f64],
        (s, n): (usize, usize),
        (from, count): (usize, usize),
        x: (f64, f64),
        area0: &[f64],
        scratch: &mut Scratch,
    ) -> bool {
        scratch.saved.clear();
        scratch.rings.clear();
        for m in 0..count {
            for &w in self.groups.with(s + (from + m) % n) {
                scratch.saved.push((w, pts[2 * w], pts[2 * w + 1]));
                scratch.rings.push(self.groups.ring[w]);
                pts[2 * w] = x.0;
                pts[2 * w + 1] = x.1;
            }
        }
        scratch.rings.sort_unstable();
        scratch.rings.dedup();
        let flattened = scratch
            .rings
            .iter()
            .any(|&k| area0[k] > MIN_AREA && self.ring_abs_area(pts, k) <= MIN_AREA);
        if flattened {
            for &(w, px, py) in scratch.saved.iter().rev() {
                pts[2 * w] = px;
                pts[2 * w + 1] = py;
            }
        }
        !flattened
    }
}

fn at(pts: &[f64], s: usize, n: usize, i: usize) -> (f64, f64) {
    let v = s + i % n;
    (pts[2 * v], pts[2 * v + 1])
}

/// Crossings between edges within reach of each other, each pair once.
fn count_ring(pts: &[f64], s: usize, e: usize) -> usize {
    let n = e - s;
    let reach = LOOKAHEAD.min(n - 2);
    let mut count = 0;
    for i in 0..n {
        let (a, b) = (at(pts, s, n, i), at(pts, s, n, i + 1));
        for d in 2..=reach {
            if i + d >= n && n - d <= reach {
                continue;
            }
            let (c, dd) = (at(pts, s, n, i + d), at(pts, s, n, i + d + 1));
            count += crossing(a, b, c, dd).is_some() as usize;
        }
    }
    count
}
