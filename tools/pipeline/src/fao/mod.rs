//! FAOSTAT bulk "All Data (Normalized)" files: one series per (domain, item, element) total.

mod areas;
pub mod fetch;
mod rules;
mod stats;

use crate::format::is_code;
use crate::index::{Indicator, Source};
use crate::wdi::{column, field, finish, reader, Reject, FIRST_YEAR};

const LAST_YEAR: u16 = 2100;
use crate::Result;
use areas::{Area, Areas};
use rules::Measure;
pub use stats::Stats;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

type Row = Vec<(f32, u32)>;

struct Acc {
    /// Position of this base unit among the units its (item, element) pair carries, in file order.
    item: String,
    element: String,
    measure: Measure,
    rows: BTreeMap<String, Row>,
    unmapped: HashMap<String, (String, u16, u16)>,
}

struct Pairs {
    seen: Vec<(String, String, Option<Measure>)>,
}

impl Pairs {
    fn measure(&mut self, code: &str, element: &str, unit: &str) -> Option<Measure> {
        let key = format!("{code}\t{unit}");
        if let Some((_, _, m)) = self.seen.iter().find(|(k, _, _)| *k == key) {
            return *m;
        }
        let m = rules::measure(element, unit);
        self.seen.push((key, unit.to_string(), m));
        m
    }
}

struct Columns {
    area: usize,
    m49: usize,
    area_name: usize,
    item_code: usize,
    item: usize,
    element_code: usize,
    element: usize,
    year: usize,
    unit: usize,
    value: usize,
    flag: usize,
    source: Option<usize>,
}

impl Columns {
    fn new(h: &csv::StringRecord) -> Result<Self> {
        Ok(Columns {
            area: column(h, "Area Code")?,
            m49: column(h, "Area Code (M49)")?,
            area_name: column(h, "Area")?,
            item_code: column(h, "Item Code")?,
            item: column(h, "Item")?,
            element_code: column(h, "Element Code")?,
            element: column(h, "Element")?,
            year: column(h, "Year")?,
            unit: column(h, "Unit")?,
            value: column(h, "Value")?,
            flag: column(h, "Flag")?,
            source: column(h, "Source Code").ok(),
        })
    }
}

/// Stores a cell; the lower source code wins. Returns false when two rows tie for the cell.
fn put(row: &mut Row, at: usize, value: f32, source: u32) -> bool {
    if row.len() <= at {
        row.resize(at + 1, (f32::NAN, u32::MAX));
    }
    match source.cmp(&row[at].1) {
        std::cmp::Ordering::Less => row[at] = (value, source),
        std::cmp::Ordering::Equal => return false,
        std::cmp::Ordering::Greater => {}
    }
    true
}

/// A missing, empty, or unparseable source code ranks below every real one.
fn source_rank(cell: Option<&str>) -> u32 {
    match cell {
        None => u32::MAX - 1,
        Some(c) => c
            .parse::<u32>()
            .ok()
            .filter(|&n| n < u32::MAX - 1)
            .unwrap_or(u32::MAX - 1),
    }
}

fn ingest(
    domain: &str,
    path: &Path,
    areas: &Areas,
    claimed: &mut HashMap<String, (String, String)>,
    stats: &mut Stats,
    min_countries: usize,
) -> Result<Vec<(Indicator, String)>> {
    let mut rdr = reader(path)?;
    let cols = Columns::new(&rdr.headers()?.clone())?;
    let mut pairs = Pairs { seen: Vec::new() };
    let mut accs: HashMap<(String, String, &'static str), Acc> = HashMap::new();
    let mut units: HashMap<(String, String), Vec<&'static str>> = HashMap::new();
    let mut refused: BTreeMap<(String, String, String), HashSet<String>> = BTreeMap::new();
    let mut kept_items: BTreeMap<(String, String, String), HashSet<String>> = BTreeMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        let (item_code, element_code) =
            (field(&rec, cols.item_code), field(&rec, cols.element_code));
        let (element, unit) = (field(&rec, cols.element), field(&rec, cols.unit));
        let pair = (
            element_code.to_string(),
            element.to_string(),
            unit.to_string(),
        );
        let Some(measure) = pairs.measure(element_code, element, unit) else {
            let items = refused.entry(pair).or_default();
            if !items.contains(item_code) {
                items.insert(item_code.to_string());
            }
            continue;
        };
        let items = kept_items.entry(pair).or_default();
        if !items.contains(item_code) {
            items.insert(item_code.to_string());
        }
        if field(&rec, cols.flag) == "F" {
            continue;
        }
        let year: u16 = match field(&rec, cols.year).parse() {
            Ok(y) if (FIRST_YEAR..=LAST_YEAR).contains(&y) => y,
            _ => {
                stats.skipped_years += 1;
                continue;
            }
        };
        let value: f64 = match field(&rec, cols.value).parse() {
            Ok(v) if f64::is_finite(v) => v,
            _ => continue,
        };
        let (code, name) = (field(&rec, cols.area), field(&rec, cols.area_name));
        let area = areas.classify(code, field(&rec, cols.m49));
        let pair_units = units
            .entry((item_code.to_string(), element_code.to_string()))
            .or_default();
        if !pair_units.contains(&measure.unit) {
            pair_units.push(measure.unit);
        }
        let key = (
            item_code.to_string(),
            element_code.to_string(),
            measure.unit,
        );
        let acc = accs.entry(key).or_insert_with(|| Acc {
            item: field(&rec, cols.item).to_string(),
            element: element.to_string(),
            measure,
            rows: BTreeMap::new(),
            unmapped: HashMap::new(),
        });
        match area {
            Area::Aggregate => {
                stats.aggregates.insert(name.to_string());
            }
            Area::Unmapped => {
                let seen =
                    acc.unmapped
                        .entry(code.to_string())
                        .or_insert((name.to_string(), year, year));
                seen.1 = seen.1.min(year);
                seen.2 = seen.2.max(year);
            }
            Area::Country(iso) => {
                if !is_code(&iso) {
                    return Err(format!("bad iso {iso:?} for area {name}").into());
                }
                if let Some((other, other_name)) =
                    claimed.insert(iso.clone(), (code.to_string(), name.to_string()))
                {
                    if other != code {
                        return Err(format!(
                            "areas {other} ({other_name}) and {code} ({name}) both map to {iso}"
                        )
                        .into());
                    }
                }
                let source = source_rank(cols.source.map(|at| field(&rec, at)));
                let scaled = (value * measure.factor) as f32;
                let row = acc.rows.entry(iso).or_default();
                if !put(row, (year - FIRST_YEAR) as usize, scaled, source) {
                    return Err(format!(
                        "{domain}: two rows for area {code} ({name}), item {item_code}, element {element_code}, year {year}"
                    )
                    .into());
                }
            }
        }
    }

    let d = stats.domains.entry(domain.to_string()).or_default();
    for (pair, items) in refused {
        d.pairs.insert(
            pair,
            stats::Pair {
                series: items.len(),
                kept: false,
            },
        );
    }
    for (pair, items) in kept_items {
        d.pairs.insert(
            pair,
            stats::Pair {
                series: items.len(),
                kept: true,
            },
        );
    }
    d.mixed_units += units.values().filter(|u| u.len() > 1).count();
    let mut keys: Vec<(String, String, &'static str)> = accs.keys().cloned().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let acc = accs.remove(&key).expect("key came from the map");
        let several = units
            .get(&(key.0.clone(), key.1.clone()))
            .is_some_and(|u| u.len() > 1);
        let width = acc.rows.values().map(Vec::len).max().unwrap_or(0);
        let rows = acc
            .rows
            .into_iter()
            .map(|(iso, mut row)| {
                row.resize(width, (f32::NAN, 0));
                (iso, row.into_iter().map(|(v, _)| v).collect())
            })
            .collect();
        match finish(rows, width, FIRST_YEAR, min_countries) {
            Ok(series) => {
                d.kept += 1;
                d.items.insert(key.0.clone());
                d.elements.insert(key.1.clone());
                for (code, (name, first, last)) in acc.unmapped {
                    let u = stats.unmapped.entry((code, name)).or_default();
                    u.first = if u.series == 0 {
                        first
                    } else {
                        u.first.min(first)
                    };
                    u.last = u.last.max(last);
                    u.series += 1;
                }
                let indicator = Indicator {
                    source: Source::Fao,
                    code: rules::slug(domain, &key.0, &key.1, several, acc.measure.unit),
                    title: rules::title(&acc.item, &acc.element),
                    unit: acc.measure.unit.to_string(),
                    series,
                    derived: None,
                };
                out.push((indicator, rules::full_title(&acc.item, &acc.element)));
            }
            Err(reject) => tally_domain(d, reject),
        }
    }
    Ok(out)
}

fn tally_domain(d: &mut stats::DomainStats, reject: Reject) {
    match reject {
        Reject::Negative => d.negative += 1,
        Reject::TooFew => d.too_few_countries += 1,
    }
}

/// Reads every domain file present under `dir` as `<CODE>.csv`.
pub fn run(dir: &Path, min_countries: usize) -> Result<(Vec<Indicator>, Stats)> {
    let areas = Areas::embedded()?;
    let mut stats = Stats::default();
    let mut claimed = HashMap::new();
    let mut candidates = Vec::new();
    for (domain, _) in fetch::FILES {
        let path = dir.join(format!("{domain}.csv"));
        if path.exists() {
            candidates.extend(ingest(
                domain,
                &path,
                &areas,
                &mut claimed,
                &mut stats,
                min_countries,
            )?);
        }
    }
    if stats.domains.is_empty() {
        return Err(format!("no FAOSTAT files in {}", dir.display()).into());
    }
    let mut seen: HashMap<(String, &str), usize> = HashMap::new();
    for (i, _) in &candidates {
        *seen.entry((i.title.clone(), i.unit.as_str())).or_default() += 1;
    }
    let clashing: HashSet<(String, String)> = seen
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|((title, unit), _)| (title, unit.to_string()))
        .collect();
    let indicators: Vec<Indicator> = candidates
        .into_iter()
        .map(|(mut i, full)| {
            if clashing.contains(&(i.title.clone(), i.unit.clone())) {
                i.title = full;
            }
            i
        })
        .collect();
    crate::catalogue::check_slugs(&indicators)?;
    Ok((indicators, stats))
}
