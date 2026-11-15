use super::Cartogram;

/// Loops that folding leaves in thin regions are short; a longer reach collapses the large side of a loop.
const LOOKAHEAD: usize = 40;
/// A collapse can expose a crossing with a neighbouring edge; later passes rarely find more.
const PASSES: usize = 4;

/// Vertices that start at one position, as runs of `members` indexed by `start`.
pub(crate) struct Groups {
    of: Vec<usize>,
    start: Vec<usize>,
    members: Vec<usize>,
}

impl Groups {
    pub(crate) fn new(pts: &[f64]) -> Self {
        let n = pts.len() / 2;
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
        Groups { of, start, members }
    }

    fn with(&self, v: usize) -> &[usize] {
        let g = self.of[v];
        &self.members[self.start[g]..self.start[g + 1]]
    }
}

fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

impl Cartogram {
    /// Pulls each loop of a self-crossing outline onto its crossing point, moving vertices that
    /// began together as one. Returns how many vertices ended up somewhere else.
    pub(crate) fn untangle(&self, pts: &mut [f64]) -> usize {
        let before = pts.to_vec();
        for _ in 0..PASSES {
            let mut changed = false;
            for k in 0..self.ring_region.len() {
                changed |= self.untangle_ring(pts, self.ring_start[k], self.ring_start[k + 1]);
            }
            if !changed {
                break;
            }
        }
        (0..pts.len() / 2)
            .filter(|&i| pts[2 * i..2 * i + 2] != before[2 * i..2 * i + 2])
            .count()
    }

    fn untangle_ring(&self, pts: &mut [f64], s: usize, e: usize) -> bool {
        let n = e - s;
        let reach = LOOKAHEAD.min(n - 2);
        let at = |pts: &[f64], i: usize| {
            let v = s + i % n;
            (pts[2 * v], pts[2 * v + 1])
        };
        let mut changed = false;
        let mut i = 0;
        while i < n {
            let (a, b) = (at(pts, i), at(pts, i + 1));
            let mut hit = None;
            for d in 2..=reach {
                let (c, dd) = (at(pts, i + d), at(pts, i + d + 1));
                let (o1, o2) = (orient(a, b, c), orient(a, b, dd));
                if o1 * o2 < 0.0 && orient(c, dd, a) * orient(c, dd, b) < 0.0 {
                    let t = o1 / (o1 - o2);
                    hit = Some((d, (c.0 + t * (dd.0 - c.0), c.1 + t * (dd.1 - c.1))));
                }
            }
            if let Some((d, x)) = hit {
                for m in 1..=d {
                    for &w in self.groups.with(s + (i + m) % n) {
                        pts[2 * w] = x.0;
                        pts[2 * w + 1] = x.1;
                    }
                }
                changed = true;
                i += d;
            }
            i += 1;
        }
        changed
    }
}
