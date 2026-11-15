use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Default, PartialEq)]
pub struct Pair {
    pub series: usize,
    pub kept: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct DomainStats {
    pub kept: usize,
    pub negative: usize,
    pub too_few_countries: usize,
    /// (item, element) pairs that carry more than one base unit.
    pub mixed_units: usize,
    pub items: BTreeSet<String>,
    pub elements: BTreeSet<String>,
    /// (element code, element, unit) with the number of items that have it.
    pub pairs: BTreeMap<(String, String, String), Pair>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Unmapped {
    pub first: u16,
    pub last: u16,
    pub series: usize,
}

#[derive(Debug, Default, PartialEq)]
pub struct Stats {
    pub domains: BTreeMap<String, DomainStats>,
    pub aggregates: BTreeSet<String>,
    /// Rows whose Year is not a single year from 1946 to 2100.
    pub skipped_years: usize,
    /// FAO areas below the aggregate range with no M49 on the map, by (code, name).
    pub unmapped: BTreeMap<(String, String), Unmapped>,
}

impl Stats {
    pub fn kept(&self) -> usize {
        self.domains.values().map(|d| d.kept).sum()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (domain, d) in &self.domains {
            write!(
                f,
                "FAO {domain}: kept {} series from {} items and {} elements, negative {}, too few countries {}, pairs with several units {}",
                d.kept,
                d.items.len(),
                d.elements.len(),
                d.negative,
                d.too_few_countries,
                d.mixed_units
            )?;
            for ((code, element, unit), p) in &d.pairs {
                let verdict = if p.kept { "kept" } else { "refused" };
                write!(
                    f,
                    "\n  {code} {element} [{unit}] {} items, {verdict}",
                    p.series
                )?;
            }
            writeln!(f)?;
        }
        write!(
            f,
            "FAO aggregate areas dropped: {}, rows with no usable year skipped: {}",
            self.aggregates.len(),
            self.skipped_years
        )?;
        for ((code, name), u) in &self.unmapped {
            write!(
                f,
                "\nFAO area without M49 on the map: {code} {name}, {}-{}, in {} kept series",
                u.first, u.last, u.series
            )?;
        }
        Ok(())
    }
}
