use crate::index::Indicator;
use crate::{catalogue, fao, wdi, Result};
use std::path::Path;

pub struct Inputs<'a> {
    pub wdi: &'a Path,
    pub fao: Option<&'a Path>,
}

/// Ingests every source given and merges them into one catalogue sorted by slug.
pub fn run(inputs: &Inputs, min_countries: usize) -> Result<(Vec<Indicator>, Vec<String>)> {
    let (mut indicators, stats) = wdi::run(inputs.wdi, min_countries)?;
    let mut reports = vec![format!("WDI: {stats}")];
    if let Some(dir) = inputs.fao {
        let (more, stats) = fao::run(dir, min_countries)?;
        reports.push(stats.to_string());
        indicators = merge(indicators, more)?;
    }
    Ok((indicators, reports))
}

/// Joins two sources' indicators, sorted by slug; a slug used twice is an error.
pub fn merge(mut a: Vec<Indicator>, b: Vec<Indicator>) -> Result<Vec<Indicator>> {
    a.extend(b);
    catalogue::check_slugs(&a)?;
    a.sort_by_key(|i| i.slug());
    Ok(a)
}
