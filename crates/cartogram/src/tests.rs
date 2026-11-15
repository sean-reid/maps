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
