use crate::format::Series;
use crate::index::Indicator;
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
    pub not_sum: usize,
    pub not_cc_by: usize,
    pub negative: usize,
    pub too_few_countries: usize,
    pub kept: usize,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "indicators {}, kept {}, dropped: no metadata {}, aggregation not a total {}, licence not CC BY 4.0 {}, negative {}, too few countries {}",
            self.total, self.kept, self.no_metadata, self.not_sum, self.not_cc_by, self.negative, self.too_few_countries
        )
    }
}

struct Meta {
    name: String,
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
    let agg = column(&headers, "Aggregation method")?;
    let lic = column(&headers, "License Type")?;
    let mut map = HashMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        map.insert(
            field(&rec, code).to_string(),
            Meta {
                name: field(&rec, name).to_string(),
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

pub fn run(input: &Path, min_countries: usize) -> Result<(Vec<Indicator>, Stats)> {
    let countries = load_countries(&input.join("WDICountry.csv"))?;
    let meta = load_meta(&input.join("WDISeries.csv"))?;

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
    for rec in rdr.records() {
        let rec = rec?;
        let code = field(&rec, code_col);
        if seen.insert(code.to_string()) {
            stats.total += 1;
            match meta.get(code) {
                None => stats.no_metadata += 1,
                Some(m) if !is_total(&m.aggregation) => stats.not_sum += 1,
                Some(m) if !is_cc_by(&m.licence) => stats.not_cc_by += 1,
                Some(_) => {
                    eligible.insert(code.to_string(), Accumulator::default());
                }
            }
        }
        let Some(acc) = eligible.get_mut(code) else {
            continue;
        };
        let country = field(&rec, country_col);
        if !countries.contains(country) {
            continue;
        }
        let mut row = Vec::with_capacity(width);
        for &(at, _) in &year_cols {
            let cell = field(&rec, at);
            row.push(if cell.is_empty() {
                f32::NAN
            } else {
                cell.parse::<f32>()
                    .map_err(|e| format!("{code} {country}: {cell:?}: {e}"))?
            });
        }
        acc.rows.insert(country.to_string(), row);
    }

    let mut indicators = Vec::new();
    for (code, acc) in eligible {
        if acc.rows.values().flatten().any(|&v| v < 0.0) {
            stats.negative += 1;
        } else if !enough_countries(&acc.rows, width, min_countries) {
            stats.too_few_countries += 1;
        } else if let Some(series) = build_series(acc.rows, first_col_year) {
            let (title, unit) = split_name(&meta[&code].name);
            indicators.push(Indicator {
                code,
                title,
                unit,
                series,
            });
            stats.kept += 1;
        } else {
            stats.too_few_countries += 1;
        }
    }
    indicators.sort_by(|a, b| a.code.cmp(&b.code));
    Ok((indicators, stats))
}

#[cfg(test)]
mod tests {
    use super::{is_cc_by, is_total, split_name};

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
}
