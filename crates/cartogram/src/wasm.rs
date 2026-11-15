use super::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Solver {
    inner: Cartogram,
    last: Option<Stats>,
}

#[wasm_bindgen]
impl Solver {
    #[wasm_bindgen(constructor)]
    pub fn new(
        lx: usize,
        ly: usize,
        n_regions: usize,
        ring_start: &[u32],
        ring_region: &[u32],
        pts: &[f64],
    ) -> Result<Solver, JsError> {
        let inner = Cartogram::new(
            lx,
            ly,
            n_regions,
            ring_start.iter().map(|&v| v as usize).collect(),
            ring_region.iter().map(|&v| v as usize).collect(),
            pts.to_vec(),
        )
        .map_err(|e| JsError::new(&e))?;
        Ok(Solver { inner, last: None })
    }

    pub fn solve(&mut self, values: &[f64]) -> Result<Vec<f64>, JsError> {
        self.keep(self.inner.solve(values, &Options::default()))
    }

    /// Continues from a previous solution; `skip_blur` drops that many blur levels.
    #[wasm_bindgen(js_name = solveFrom)]
    pub fn solve_from(
        &mut self,
        values: &[f64],
        start: &[f64],
        skip_blur: usize,
    ) -> Result<Vec<f64>, JsError> {
        self.keep(
            self.inner
                .solve_from(values, &Options::default(), start, skip_blur),
        )
    }

    /// Area-weighted relative error of the last solve.
    #[wasm_bindgen(js_name = meanError)]
    pub fn mean_error(&self) -> f64 {
        self.last.as_ref().map_or(f64::NAN, |s| s.mean_err)
    }

    /// Whether the last solve cut an advection short.
    pub fn truncated(&self) -> bool {
        self.last.as_ref().is_some_and(|s| s.truncated)
    }

    fn keep(&mut self, r: Result<(Vec<f64>, Stats), String>) -> Result<Vec<f64>, JsError> {
        let (pts, stats) = r.map_err(|e| JsError::new(&e))?;
        self.last = Some(stats);
        Ok(pts)
    }
}
