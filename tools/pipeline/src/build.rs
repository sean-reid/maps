use crate::index::Indicator;
use crate::{wdi, Result};
use std::path::Path;

pub struct Inputs<'a> {
    pub wdi: &'a Path,
}

/// Ingests every source given; returns the indicators sorted by slug and one report per source.
pub fn run(inputs: &Inputs, min_countries: usize) -> Result<(Vec<Indicator>, Vec<String>)> {
    let (indicators, stats) = wdi::run(inputs.wdi, min_countries)?;
    Ok((indicators, vec![format!("WDI: {stats}")]))
}
