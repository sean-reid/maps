use super::*;

fn square(lx: usize, ly: usize) -> Cartogram {
    let pts = vec![2.0, 2.0, 6.0, 2.0, 6.0, 6.0, 2.0, 6.0];
    Cartogram::new(lx, ly, 1, vec![0, 4], vec![0], pts).unwrap()
}

#[test]
fn poisson_inverts_the_neumann_laplacian_and_returns_the_mean() {
    let (lx, ly) = (16, 12);
    let c = square(lx, ly);
    let src: Vec<f64> = (0..lx * ly)
        .map(|k| 1.0 + ((k * 37 % 101) as f64) / 101.0)
        .collect();
    let mean = src.iter().sum::<f64>() / src.len() as f64;
    let mut rho = src.clone();
    let mut phi = vec![0.0; lx * ly];
    let got = c.poisson(&mut rho, &mut phi, 0.0);
    assert!((got - mean).abs() < 1e-12);
    for k in 0..lx * ly {
        assert!((rho[k] - src[k]).abs() < 1e-9, "unblurred rho changed");
        let (i, j) = (k % lx, k / lx);
        let at = |di: isize, dj: isize| {
            let x = (i as isize + di).clamp(0, lx as isize - 1) as usize;
            let y = (j as isize + dj).clamp(0, ly as isize - 1) as usize;
            phi[y * lx + x]
        };
        let lap = at(-1, 0) + at(1, 0) + at(0, -1) + at(0, 1) - 4.0 * phi[k];
        assert!((lap - (src[k] - mean)).abs() < 1e-9, "cell {k}: {lap}");
    }
}

#[test]
fn blur_keeps_the_mean() {
    let c = square(16, 12);
    let mut rho: Vec<f64> = (0..16 * 12).map(|k| (k % 7) as f64).collect();
    let before = rho.iter().sum::<f64>();
    let mut phi = vec![0.0; rho.len()];
    c.poisson(&mut rho, &mut phi, 2.0);
    assert!((rho.iter().sum::<f64>() - before).abs() < 1e-9);
}

#[test]
fn rasterize_fills_cell_centres_inside_and_leaves_holes_and_sea_alone() {
    let outer = [1.0, 1.0, 9.0, 1.0, 9.0, 9.0, 1.0, 9.0];
    let hole = [4.0, 4.0, 4.0, 6.0, 6.0, 6.0, 6.0, 4.0];
    let pts = [outer, hole].concat();
    let c = Cartogram::new(12, 12, 1, vec![0, 4, 8], vec![0, 0], pts.clone()).unwrap();
    let mut rho = vec![0.0; 144];
    c.rasterize(&pts, &[5.0], &mut rho);
    let at = |x: usize, y: usize| rho[y * 12 + x];
    assert_eq!(at(1, 1), 5.0);
    assert_eq!(at(8, 8), 5.0);
    assert_eq!(at(0, 0), 1.0);
    assert_eq!(at(9, 9), 1.0);
    assert_eq!(at(4, 4), 1.0);
    assert_eq!(at(5, 5), 1.0);
    assert_eq!(at(3, 5), 5.0);
    assert_eq!(rho.iter().filter(|&&v| v == 5.0).count(), 64 - 4);
}

#[test]
fn bad_input_is_refused() {
    let pts = vec![2.0, 2.0, 6.0, 2.0, 6.0, 6.0, 2.0, 6.0];
    let new = |lx, starts: Vec<usize>, regions: Vec<usize>, p: Vec<f64>| {
        Cartogram::new(lx, 8, 1, starts, regions, p).is_err()
    };
    assert!(new(1, vec![0, 4], vec![0], pts.clone()));
    assert!(new(8, vec![0, 2], vec![0], pts[..4].to_vec()));
    assert!(new(8, vec![0, 4], vec![1], pts.clone()));
    assert!(new(8, vec![0, 4], vec![0], pts[..6].to_vec()));
    let mut nan = pts.clone();
    nan[3] = f64::NAN;
    assert!(new(8, vec![0, 4], vec![0], nan));
    let c = square(8, 8);
    assert!(c.solve(&[1.0, 2.0], &Options::default()).is_err());
    assert!(c
        .solve_from(&[1.0], &Options::default(), &pts[..6], 0)
        .is_err());
}

#[test]
fn a_region_without_rings_does_not_block_convergence() {
    let pts = vec![2.0, 2.0, 6.0, 2.0, 6.0, 6.0, 2.0, 6.0];
    let c = Cartogram::new(8, 8, 2, vec![0, 4], vec![0], pts).unwrap();
    let (_, stats) = c.solve(&[1.0, 1.0], &Options::default()).unwrap();
    assert_eq!(stats.iters, 0);
    assert!(stats.max_err < 0.01);
}

#[test]
fn a_huge_blur_skip_stays_finite() {
    let c = square(8, 8);
    let start = c.points().to_vec();
    let (out, _) = c
        .solve_from(&[1.0], &Options::default(), &start, usize::MAX / 2)
        .unwrap();
    assert!(out.iter().all(|v| v.is_finite()));
}

fn proper_crossings(ring: &[f64]) -> usize {
    let n = ring.len() / 2;
    let p = |i: usize| (ring[2 * (i % n)], ring[2 * (i % n) + 1]);
    let o = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    };
    let mut count = 0;
    for i in 0..n {
        for j in i + 2..n {
            if (j + 1) % n == i {
                continue;
            }
            let (a, b, c, d) = (p(i), p(i + 1), p(j), p(j + 1));
            if o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0 {
                count += 1;
            }
        }
    }
    count
}

fn notched_square() -> Vec<f64> {
    vec![
        2.0, 2.0, 18.0, 2.0, 18.0, 18.0, 14.0, 18.0, 8.0, 18.0, 10.0, 20.0, 11.0, 16.0, 6.0, 18.0,
        2.0, 18.0,
    ]
}

fn single(pts: Vec<f64>) -> Cartogram {
    Cartogram::new(24, 24, 1, vec![0, pts.len() / 2], vec![0], pts).unwrap()
}

#[test]
fn a_notch_that_crosses_itself_is_pulled_to_the_crossing() {
    let pts = notched_square();
    let c = single(pts.clone());
    assert_eq!(proper_crossings(&pts), 1);
    let mut out = pts.clone();
    let (moved, left) = c.untangle(&mut out);
    assert_eq!(left, 0);
    assert_eq!(proper_crossings(&out), 0);
    assert_eq!(moved, 2);
    let (before, after) = (ring_area(&pts).abs(), ring_area(&out).abs());
    assert!((after - before).abs() < 0.02 * before, "{before} {after}");
}

#[test]
fn untangling_twice_changes_nothing_more() {
    let c = single(notched_square());
    let mut once = c.points().to_vec();
    c.untangle(&mut once);
    let mut twice = once.clone();
    assert_eq!(c.untangle(&mut twice), (0, 0));
    assert_eq!(once, twice);
}

#[test]
fn a_loop_on_a_shared_border_keeps_both_sides_identical() {
    let border = [
        (10.0, 2.0),
        (10.0, 6.0),
        (8.0, 11.0),
        (12.0, 10.0),
        (10.0, 8.0),
        (10.0, 14.0),
        (10.0, 18.0),
    ];
    let left: Vec<(f64, f64)> = [(2.0, 2.0)]
        .into_iter()
        .chain(border)
        .chain([(2.0, 18.0)])
        .collect();
    let right: Vec<(f64, f64)> = [(10.0, 18.0), (18.0, 18.0), (18.0, 2.0)]
        .into_iter()
        .chain(border[..6].iter().copied())
        .collect();
    let (la, rb) = (left.len(), right.len());
    let pts: Vec<f64> = left.iter().chain(&right).flat_map(|p| [p.0, p.1]).collect();
    let c = Cartogram::new(24, 24, 2, vec![0, la, la + rb], vec![0, 1], pts.clone()).unwrap();
    assert_eq!(proper_crossings(&pts[..2 * la]), 1);
    assert_eq!(proper_crossings(&pts[2 * la..]), 1);
    let mut out = pts.clone();
    assert!(c.untangle(&mut out).0 > 0);
    assert_eq!(proper_crossings(&out[..2 * la]), 0);
    assert_eq!(proper_crossings(&out[2 * la..]), 0);
    let n = pts.len() / 2;
    let mut shared = 0;
    for i in 0..n {
        for j in i + 1..n {
            if pts[2 * i..2 * i + 2] == pts[2 * j..2 * j + 2] {
                assert_eq!(out[2 * i..2 * i + 2], out[2 * j..2 * j + 2]);
                shared += 1;
            }
        }
    }
    assert_eq!(shared, 7);
}

#[test]
fn clean_rings_and_solves_are_returned_untouched() {
    let c = square(8, 8);
    let mut out = c.points().to_vec();
    assert_eq!(c.untangle(&mut out), (0, 0));
    assert_eq!(out, c.points());
    let (solved, stats) = c.solve(&[1.0], &Options::default()).unwrap();
    assert_eq!(stats.collapsed, 0);
    assert_eq!(solved, c.points());
}

#[test]
fn a_loop_longer_than_the_look_ahead_is_left_alone() {
    let pts: Vec<f64> = (0..100)
        .flat_map(|k| {
            let t = std::f64::consts::TAU * (k as f64 + 0.5) / 100.0;
            [30.0 + 25.0 * t.cos(), 30.0 + 25.0 * t.sin() * t.cos()]
        })
        .collect();
    let c = single(pts.clone());
    assert_eq!(proper_crossings(&pts), 1);
    let mut out = pts.clone();
    assert_eq!(c.untangle(&mut out), (0, 0));
    assert_eq!(out, pts);
}

#[test]
fn the_shorter_arc_of_a_loop_is_the_one_collapsed() {
    use std::f64::consts::PI;
    let pts: Vec<f64> = (0..50)
        .flat_map(|k| {
            let r = (k + 50 - 41) % 50;
            let t = if r < 12 {
                PI / 2.0 + PI / 24.0 + r as f64 * PI / 12.0
            } else {
                1.5 * PI + PI / 24.0 + (r - 12) as f64 * 11.0 * PI / 444.0
            };
            [30.0 + 25.0 * t.cos(), 30.0 + 25.0 * t.sin() * t.cos()]
        })
        .collect();
    let c = single(pts.clone());
    assert_eq!(proper_crossings(&pts), 1);
    let mut out = pts.clone();
    let (moved, left) = c.untangle(&mut out);
    assert_eq!((moved, left), (12, 0));
    for k in 0..50 {
        let short = k >= 41 || k <= 2;
        assert_eq!(
            out[2 * k..2 * k + 2] != pts[2 * k..2 * k + 2],
            short,
            "vertex {k}"
        );
    }
}

#[test]
fn a_region_that_lost_all_its_area_counts_as_a_full_error() {
    assert_eq!(errors(&[0.0, 4.0], &[2.0, 4.0]), (1.0, 1.0 * 2.0 / 6.0));
    assert_eq!(errors(&[0.0, 4.0], &[0.0, 4.0]), (0.0, 0.0));
}

#[test]
fn a_collapse_that_would_empty_another_ring_is_skipped() {
    let a: [(f64, f64); 10] = [
        (2.0, 2.0),
        (18.0, 2.0),
        (18.0, 18.0),
        (14.0, 18.0),
        (8.0, 18.0),
        (9.0, 20.0),
        (10.0, 19.0),
        (11.0, 16.0),
        (6.0, 18.0),
        (2.0, 18.0),
    ];
    let islet = [a[4], a[5], a[6]];
    let pts: Vec<f64> = a.iter().chain(&islet).flat_map(|p| [p.0, p.1]).collect();
    let c = Cartogram::new(24, 24, 2, vec![0, 10, 13], vec![0, 1], pts.clone()).unwrap();
    assert_eq!(proper_crossings(&pts[..20]), 1);
    let mut out = pts.clone();
    assert_eq!(c.untangle(&mut out), (0, 1));
    assert_eq!(out, pts);
}

#[test]
fn a_solve_that_folds_comes_back_untangled() {
    let (k, cw, ch) = (3usize, 16.0, 8.0);
    let mut starts = vec![0];
    let mut pts = Vec::new();
    for r in 0..k * k {
        let (a, c) = (8.0 + (r % k) as f64 * cw, 4.0 + (r / k) as f64 * ch);
        let corners = [(a, c), (a + cw, c), (a + cw, c + ch), (a, c + ch)];
        for q in 0..4 {
            let (p, e) = (corners[q], corners[(q + 1) % 4]);
            let steps = ((e.0 - p.0).abs().max((e.1 - p.1).abs()) / 0.5) as usize;
            for s in 0..steps {
                let t = s as f64 / steps as f64;
                pts.extend([p.0 + t * (e.0 - p.0), p.1 + t * (e.1 - p.1)]);
            }
        }
        starts.push(pts.len() / 2);
    }
    let c = Cartogram::new(64, 32, k * k, starts.clone(), (0..k * k).collect(), pts).unwrap();
    let values = [8.0, 2.0, 4920.0, 46099.0, 1972.0, 584.0, 231.0, 436.0, 57.0];
    let (out, stats) = c.solve(&values, &Options::default()).unwrap();
    assert!(stats.collapsed > 0);
    assert_eq!(stats.crossings, 0);
    for w in starts.windows(2) {
        assert_eq!(proper_crossings(&out[2 * w[0]..2 * w[1]]), 0);
    }
}
