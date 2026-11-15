use cartogram::{ring_area, Cartogram, Options};

/// A row of `n` unit-height rectangles tiling a 32x16 block centred in a 64x32 grid,
/// each edge split into one-cell segments.
fn strip(n: usize) -> Cartogram {
    let (x0, y0, w, h) = (16.0, 8.0, 32.0 / n as f64, 16.0);
    let mut starts = vec![0];
    let mut pts = Vec::new();
    for r in 0..n {
        let (a, b) = (x0 + r as f64 * w, x0 + (r + 1) as f64 * w);
        let corners: [(f64, f64); 4] = [(a, y0), (b, y0), (b, y0 + h), (a, y0 + h)];
        for k in 0..4 {
            let (p, q) = (corners[k], corners[(k + 1) % 4]);
            let steps = ((q.0 - p.0).abs().max((q.1 - p.1).abs())).ceil() as usize;
            for s in 0..steps {
                let t = s as f64 / steps as f64;
                pts.extend([p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)]);
            }
        }
        starts.push(pts.len() / 2);
    }
    Cartogram::new(64, 32, n, starts, (0..n).collect(), pts)
}

#[test]
fn equal_values_on_equal_regions_move_nothing() {
    let c = strip(4);
    let (out, stats) = c.solve(&[1.0; 4], &Options::default());
    assert_eq!(stats.iters, 0);
    assert_eq!(out, c.points());
}

#[test]
fn areas_reach_their_share_and_total_is_kept() {
    let c = strip(2);
    let opt = Options {
        max_iter: 20,
        ..Options::default()
    };
    let (out, stats) = c.solve(&[3.0, 1.0], &opt);
    let (before, after) = (c.areas(c.points()), c.areas(&out));
    let total: f64 = before.iter().sum();
    assert!(stats.max_err < 0.02, "max_err {}", stats.max_err);
    assert!((after[0] / total - 0.75).abs() < 0.02);
    assert!((after.iter().sum::<f64>() / total - 1.0).abs() < 0.02);
}

#[test]
fn shared_border_vertices_stay_shared() {
    let c = strip(2);
    let (out, _) = c.solve(&[5.0, 1.0], &Options::default());
    let src = c.points();
    let n = src.len() / 2;
    let mut pairs = 0;
    for i in 0..n {
        for j in i + 1..n {
            if src[2 * i] == src[2 * j] && src[2 * i + 1] == src[2 * j + 1] {
                assert_eq!((out[2 * i], out[2 * i + 1]), (out[2 * j], out[2 * j + 1]));
                pairs += 1;
            }
        }
    }
    assert!(pairs > 10);
}

#[test]
fn rings_keep_their_orientation() {
    let c = strip(4);
    let (out, _) = c.solve(&[40.0, 1.0, 9.0, 0.0], &Options::default());
    let per = out.len() / 4;
    for r in 0..4 {
        let (a, b) = (
            &c.points()[r * per..(r + 1) * per],
            &out[r * per..(r + 1) * per],
        );
        assert_eq!(ring_area(a).signum(), ring_area(b).signum());
    }
}

#[test]
fn missing_and_tiny_values_get_floors() {
    let c = strip(4);
    let opt = Options::default();
    let own = c.areas(c.points())[0];
    let t = c.targets(&[1.0, f64::NAN, 0.0, 1.0], &opt);
    let scale = t.iter().sum::<f64>() / (4.0 * own);
    assert!((scale - 1.0).abs() < 1e-9);
    assert!(t[1] >= opt.missing_frac * own * 0.99 && t[1] < 0.2 * own);
    assert!(t[2] >= opt.min_ratio * own * 0.99 && t[2] < t[1]);
    assert!((t[0] - t[3]).abs() < 1e-9);
}

#[test]
fn warm_start_from_a_neighbouring_solution_converges_in_fewer_steps() {
    let c = strip(3);
    let opt = Options {
        max_iter: 20,
        ..Options::default()
    };
    let (base, cold) = c.solve(&[4.0, 2.0, 1.0], &opt);
    let (_, warm) = c.solve_from(&[4.2, 2.0, 1.0], &opt, &base, 3);
    assert!(warm.max_err < 0.02);
    assert!(
        warm.iters < cold.iters,
        "warm {} cold {}",
        warm.iters,
        cold.iters
    );
}
