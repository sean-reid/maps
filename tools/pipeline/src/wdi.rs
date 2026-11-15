use crate::derive::{self, Rows};
use crate::format::{is_code, Series};
use crate::index::Indicator;
use crate::rates::{self, Plan};
use crate::Result;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::path::Path;

pub const DEFAULT_MIN_COUNTRIES: usize = 40;
pub const FIRST_YEAR: u16 = 1946;
/// Aggregation methods the World Bank uses for quantities that add up across countries.
const TOTALS: [&str; 2] = ["Sum", "Gap-filled total"];

fn is_total(aggregation: &str) -> bool {
    TOTALS.contains(&aggregation.trim())
}

/// The licence column spells it both "CC BY-4.0" and "CC BY 4.0".
fn is_cc_by(licence: &str) -> bool {
    licence.trim().replace('-', " ") == "CC BY 4.0"
}

#[derive(Debug, Default, PartialEq)]
pub struct Stats {
    pub total: usize,
    pub no_metadata: usize,
    pub no_rule: usize,
    pub not_cc_by: usize,
    pub negative: usize,
    pub too_few_countries: usize,
    pub duplicate: usize,
    pub kept: usize,
    pub derived: usize,
    pub twins: usize,
    pub by_rule: BTreeMap<&'static str, usize>,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "indicators {}, kept {} native and {} derived, dropped: no metadata {}, not a total and no rule {}, licence not CC BY 4.0 {}, negative {}, too few countries {}, derived title already a native total {}; derived pairs sharing title and unit {}",
            self.total, self.kept, self.derived, self.no_metadata, self.no_rule, self.not_cc_by, self.negative, self.too_few_countries, self.duplicate, self.twins
        )?;
        for (rule, n) in &self.by_rule {
            write!(f, "\n  {rule} {n}")?;
        }
        Ok(())
    }
}

struct Meta {
    name: String,
    definition: String,
    aggregation: String,
    licence: String,
}

#[derive(Default)]
struct Accumulator {
    rows: BTreeMap<String, Vec<f32>>,
}

fn reader(path: &Path) -> Result<csv::Reader<File>> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(csv::ReaderBuilder::new().flexible(true).from_reader(file))
}

fn column(headers: &csv::StringRecord, name: &str) -> Result<usize> {
    let name_at = |h: &str| h.trim_start_matches('\u{feff}').trim() == name;
    headers
        .iter()
        .position(name_at)
        .ok_or_else(|| format!("missing column {name}").into())
}

fn parse_cell(cell: &str) -> f32 {
    match cell.parse::<f32>() {
        Ok(v) if v.is_finite() => v,
        _ => f32::NAN,
    }
}

fn parse_wide(cell: &str) -> f64 {
    match cell.parse::<f64>() {
        Ok(v) if v.is_finite() => v,
        _ => f64::NAN,
    }
}

fn field(record: &csv::StringRecord, at: usize) -> &str {
    record.get(at).unwrap_or("").trim()
}

fn load_countries(path: &Path) -> Result<HashSet<String>> {
    let mut rdr = reader(path)?;
    let headers = rdr.headers()?.clone();
    let (code, region) = (
        column(&headers, "Country Code")?,
        column(&headers, "Region")?,
    );
    let mut set = HashSet::new();
    for rec in rdr.records() {
        let rec = rec?;
        if !field(&rec, region).is_empty() {
            set.insert(field(&rec, code).to_string());
        }
    }
    Ok(set)
}

fn load_meta(path: &Path) -> Result<HashMap<String, Meta>> {
    let mut rdr = reader(path)?;
    let headers = rdr.headers()?.clone();
    let code = column(&headers, "Series Code")?;
    let name = column(&headers, "Indicator Name")?;
    let definition = column(&headers, "Long definition")?;
    let agg = column(&headers, "Aggregation method")?;
    let lic = column(&headers, "License Type")?;
    let mut map = HashMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        map.insert(
            field(&rec, code).to_string(),
            Meta {
                name: field(&rec, name).to_string(),
                definition: field(&rec, definition).to_string(),
                aggregation: field(&rec, agg).to_string(),
                licence: field(&rec, lic).to_string(),
            },
        );
    }
    Ok(map)
}

/// Splits "Title (unit)" into ("Title", "unit"); no trailing parenthesis gives an empty unit.
pub fn split_name(name: &str) -> (String, String) {
    let name = name.trim();
    if let Some(body) = name.strip_suffix(')') {
        let mut depth = 1;
        for (at, c) in body.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return (
                    body[..at].trim().to_string(),
                    body[at + 1..].trim().to_string(),
                );
            }
        }
    }
    (name.to_string(), String::new())
}

fn build_series(rows: BTreeMap<String, Vec<f32>>, first_col_year: u16) -> Option<Series> {
    let rows: Vec<(String, Vec<f32>)> = rows
        .into_iter()
        .filter(|(_, v)| v.iter().any(|x| x.is_finite()))
        .collect();
    let width = rows.first()?.1.len();
    let any = |y: usize| rows.iter().any(|(_, v)| v[y].is_finite());
    let lo = (0..width).find(|&y| any(y))?;
    let hi = (0..width).rev().find(|&y| any(y))?;
    let mut values = Vec::with_capacity(rows.len() * (hi - lo + 1));
    let mut codes = Vec::with_capacity(rows.len());
    for (code, v) in rows {
        values.extend_from_slice(&v[lo..=hi]);
        codes.push(code);
    }
    Some(Series {
        codes,
        first_year: first_col_year + lo as u16,
        years: (hi - lo + 1) as u16,
        values,
    })
}

fn enough_countries(rows: &BTreeMap<String, Vec<f32>>, width: usize, min: usize) -> bool {
    (0..width).any(|y| rows.values().filter(|v| v[y].is_finite()).count() >= min)
}

enum Reject {
    Negative,
    TooFew,
}

fn finish(
    rows: BTreeMap<String, Vec<f32>>,
    width: usize,
    first_year: u16,
    min: usize,
) -> std::result::Result<Series, Reject> {
    // Rows only hold years from FIRST_YEAR on, so earlier negatives never count.
    if rows.values().flatten().any(|&v| v < 0.0) {
        return Err(Reject::Negative);
    }
    if !enough_countries(&rows, width, min) {
        return Err(Reject::TooFew);
    }
    build_series(rows, first_year).ok_or(Reject::TooFew)
}

fn tally(stats: &mut Stats, reject: Reject) {
    match reject {
        Reject::Negative => stats.negative += 1,
        Reject::TooFew => stats.too_few_countries += 1,
    }
}

/// A native total with the same title wins when its unit matches or is not stated.
fn is_native(native: &HashSet<(String, String)>, plan: &Plan) -> bool {
    let key = |unit: &str| (plan.title.clone(), unit.to_string());
    native.contains(&key(&plan.unit)) || native.contains(&key(""))
}

/// Counts pairs of derived series that share a title and unit.
fn twin_pairs(derived: &[Indicator]) -> usize {
    let mut groups: HashMap<(&str, &str), usize> = HashMap::new();
    for i in derived {
        *groups.entry((&i.title, &i.unit)).or_default() += 1;
    }
    groups.values().map(|n| n * (n - 1) / 2).sum()
}

pub fn run(input: &Path, min_countries: usize) -> Result<(Vec<Indicator>, Stats)> {
    let countries = load_countries(&input.join("WDICountry.csv"))?;
    let meta = load_meta(&input.join("WDISeries.csv"))?;
    let plans: HashMap<String, Plan> = meta
        .iter()
        .filter(|(_, m)| !is_total(&m.aggregation) && is_cc_by(&m.licence))
        .filter_map(|(code, m)| Some((code.clone(), rates::plan(&m.name, &m.definition)?)))
        .collect();
    let needed: HashSet<&str> = plans
        .values()
        .flat_map(|p| p.rule.denom.iter().copied())
        .chain(plans.keys().map(String::as_str))
        .collect();

    let mut rdr = reader(&input.join("WDICSV.csv"))?;
    let headers = rdr.headers()?.clone();
    let country_col = column(&headers, "Country Code")?;
    let code_col = column(&headers, "Indicator Code")?;
    let year_cols: Vec<(usize, u16)> = headers
        .iter()
        .enumerate()
        .filter_map(|(at, h)| Some((at, h.trim().parse::<u16>().ok()?)))
        .filter(|&(_, y)| y >= FIRST_YEAR)
        .collect();
    let first_col_year = year_cols.first().ok_or("no year columns")?.1;
    let width = year_cols.len();
    if year_cols.windows(2).any(|w| w[1].1 != w[0].1 + 1) {
        return Err("year columns are not consecutive".into());
    }

    let mut stats = Stats::default();
    let mut seen: HashSet<String> = HashSet::new();
    let mut eligible: HashMap<String, Accumulator> = HashMap::new();
    let mut wide: HashMap<String, Rows> = HashMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        let code = field(&rec, code_col);
        if seen.insert(code.to_string()) {
            stats.total += 1;
            match meta.get(code) {
                None => stats.no_metadata += 1,
                Some(m) if is_total(&m.aggregation) && !is_cc_by(&m.licence) => {
                    stats.not_cc_by += 1
                }
                Some(m) if is_total(&m.aggregation) => {
                    eligible.insert(code.to_string(), Accumulator::default());
                }
                Some(_) if plans.contains_key(code) => {}
                Some(m) if rates::plan(&m.name, &m.definition).is_some() => stats.not_cc_by += 1,
                Some(_) => stats.no_rule += 1,
            }
        }
        let country = field(&rec, country_col);
        if !is_code(country) || !countries.contains(country) {
            continue;
        }
        let native = eligible.get_mut(code);
        let derived = needed.contains(code);
        if native.is_none() && !derived {
            continue;
        }
        let cells: Vec<&str> = year_cols.iter().map(|&(at, _)| field(&rec, at)).collect();
        if let Some(acc) = native {
            acc.rows.insert(
                country.to_string(),
                cells.iter().map(|c| parse_cell(c)).collect(),
            );
        }
        if derived {
            let row = cells.iter().map(|c| parse_wide(c)).collect();
            wide.entry(code.to_string())
                .or_default()
                .insert(country.to_string(), row);
        }
    }

    let mut indicators = Vec::new();
    for (code, acc) in eligible {
        match finish(acc.rows, width, first_col_year, min_countries) {
            Ok(series) => {
                let (title, unit) = split_name(&meta[&code].name);
                indicators.push(Indicator {
                    code,
                    title,
                    unit,
                    series,
                    derived: None,
                });
                stats.kept += 1;
            }
            Err(reject) => tally(&mut stats, reject),
        }
    }
    let native: HashSet<(String, String)> = indicators
        .iter()
        .map(|i| (i.title.clone(), i.unit.clone()))
        .collect();
    let mut codes: Vec<&String> = plans.keys().collect();
    codes.sort();
    let mut derived = Vec::new();
    for code in codes {
        let plan = &plans[code];
        if is_native(&native, plan) {
            stats.duplicate += 1;
            continue;
        }
        let rows = match wide.get(code) {
            Some(rate) => derive::product(plan.rule, rate, &wide),
            None => BTreeMap::new(),
        };
        match finish(rows, width, first_col_year, min_countries) {
            Ok(series) => {
                *stats.by_rule.entry(plan.rule.name).or_default() += 1;
                derived.push(Indicator {
                    code: code.clone(),
                    title: plan.title.clone(),
                    unit: plan.unit.clone(),
                    series,
                    derived: Some(plan.rule.note.to_string()),
                });
            }
            Err(reject) => tally(&mut stats, reject),
        }
    }
    stats.derived = derived.len();
    stats.twins = twin_pairs(&derived);
    indicators.extend(derived);
    indicators.sort_by_key(|i| i.slug());
    Ok((indicators, stats))
}

#[cfg(test)]
mod tests {
    use super::{is_cc_by, is_total, parse_cell, split_name};

    #[test]
    fn name_with_unit() {
        assert_eq!(
            split_name("Land area (sq. km)"),
            ("Land area".into(), "sq. km".into())
        );
    }

    #[test]
    fn name_without_unit() {
        assert_eq!(
            split_name("Population, total"),
            ("Population, total".into(), String::new())
        );
    }

    #[test]
    fn nested_and_inner_parentheses() {
        assert_eq!(
            split_name("Land (area) of things (a (b) c)"),
            ("Land (area) of things".into(), "a (b) c".into())
        );
    }

    #[test]
    fn parenthesis_only_inside_is_not_a_unit() {
        assert_eq!(
            split_name("Imports (goods) of trade"),
            ("Imports (goods) of trade".into(), String::new())
        );
    }

    #[test]
    fn both_kinds_of_total_pass_and_averages_do_not() {
        assert!(is_total("Sum"));
        assert!(is_total("Gap-filled total"));
        assert!(!is_total("Weighted average"));
        assert!(!is_total(""));
    }

    #[test]
    fn both_spellings_of_the_licence_pass() {
        assert!(is_cc_by("CC BY-4.0"));
        assert!(is_cc_by("CC BY 4.0"));
        assert!(!is_cc_by("SIPRI terms and conditions"));
    }

    #[test]
    fn unusable_cells_are_missing() {
        assert_eq!(parse_cell("2.5"), 2.5);
        for cell in ["..", "abc", "1e99", "-1e99", "NaN", "inf"] {
            assert!(parse_cell(cell).is_nan(), "{cell}");
        }
    }
}
