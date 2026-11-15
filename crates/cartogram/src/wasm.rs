//! C ABI for the browser. The caller owns every buffer it allocates here and
//! passes lengths in elements; a failed call leaves its message in `error_ptr`.

use super::{Cartogram, Options, Stats};
use std::cell::RefCell;

pub struct Solver {
    inner: Cartogram,
    points: usize,
    last: Option<Stats>,
}

thread_local! {
    static ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn fail(message: String) {
    ERROR.with(|e| *e.borrow_mut() = message);
}

#[no_mangle]
pub extern "C" fn error_ptr() -> *const u8 {
    ERROR.with(|e| e.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn error_len() -> usize {
    ERROR.with(|e| e.borrow().len())
}

/// Allocates room for `len` f64 values, which also aligns u32 data.
#[no_mangle]
pub extern "C" fn alloc_f64(len: usize) -> *mut f64 {
    let mut buffer = Vec::<f64>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
/// `ptr` and `len` are those of an earlier `alloc_f64` call, freed once.
#[no_mangle]
pub unsafe extern "C" fn free_f64(ptr: *mut f64, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Returns null on bad input.
///
/// # Safety
/// `ring_start` holds `rings + 1` values, `ring_region` holds `rings`, `pts` holds `2 * points`.
#[no_mangle]
pub unsafe extern "C" fn solver_new(
    lx: usize,
    ly: usize,
    regions: usize,
    rings: usize,
    ring_start: *const u32,
    ring_region: *const u32,
    points: usize,
    pts: *const f64,
) -> *mut Solver {
    let starts = std::slice::from_raw_parts(ring_start, rings + 1);
    let owners = std::slice::from_raw_parts(ring_region, rings);
    let pts = std::slice::from_raw_parts(pts, 2 * points);
    match Cartogram::new(
        lx,
        ly,
        regions,
        starts.iter().map(|&v| v as usize).collect(),
        owners.iter().map(|&v| v as usize).collect(),
        pts.to_vec(),
    ) {
        Ok(inner) => Box::into_raw(Box::new(Solver {
            inner,
            points,
            last: None,
        })),
        Err(message) => {
            fail(message);
            std::ptr::null_mut()
        }
    }
}

/// Solves for `values` into `out`, continuing from `start` when it is not null. Returns 0 on success.
///
/// # Safety
/// `solver` came from `solver_new`; `values` holds one f64 per region; `start` (when not null)
/// and `out` hold `2 * points` f64 each.
#[no_mangle]
pub unsafe extern "C" fn solver_solve(
    solver: *mut Solver,
    values: *const f64,
    regions: usize,
    start: *const f64,
    skip_blur: usize,
    out: *mut f64,
) -> i32 {
    let solver = &mut *solver;
    let values = std::slice::from_raw_parts(values, regions);
    let opt = Options::default();
    let result = if start.is_null() {
        solver.inner.solve(values, &opt)
    } else {
        let start = std::slice::from_raw_parts(start, 2 * solver.points);
        solver.inner.solve_from(values, &opt, start, skip_blur)
    };
    match result {
        Ok((pts, stats)) => {
            std::slice::from_raw_parts_mut(out, pts.len()).copy_from_slice(&pts);
            solver.last = Some(stats);
            0
        }
        Err(message) => {
            fail(message);
            1
        }
    }
}

/// Area-weighted relative error of the last solve.
///
/// # Safety
/// `solver` came from `solver_new`.
#[no_mangle]
pub unsafe extern "C" fn solver_mean_error(solver: *const Solver) -> f64 {
    (*solver).last.as_ref().map_or(f64::NAN, |s| s.mean_err)
}

/// Whether the last solve cut an advection short.
///
/// # Safety
/// `solver` came from `solver_new`.
#[no_mangle]
pub unsafe extern "C" fn solver_truncated(solver: *const Solver) -> i32 {
    (*solver).last.as_ref().is_some_and(|s| s.truncated) as i32
}
