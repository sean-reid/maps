//! Multiplies a rate by its denominator, country by country and year by year.

use crate::rates::Rule;
use std::collections::{BTreeMap, HashMap};

/// Rows of f64 values by country code, one value per year column.
pub type Rows = BTreeMap<String, Vec<f64>>;

fn fits(v: f64) -> f32 {
    let v = v as f32;
    if v.is_finite() {
        v
    } else {
        f32::NAN
    }
}

/// A product exists only where the rate and every denominator part are finite.
pub fn product(
    rule: &Rule,
    rate: &Rows,
    denominators: &HashMap<String, Rows>,
) -> BTreeMap<String, Vec<f32>> {
    let parts: Option<Vec<&Rows>> = rule.denom.iter().map(|c| denominators.get(*c)).collect();
    let mut out = BTreeMap::new();
    let Some(parts) = parts else {
        return out;
    };
    for (country, rates) in rate {
        let rows: Option<Vec<&Vec<f64>>> = parts.iter().map(|p| p.get(country)).collect();
        let Some(rows) = rows else { continue };
        let values = (0..rates.len())
            .map(|y| {
                let base: f64 = rows.iter().map(|r| r[y]).sum();
                fits(rates[y] * base * rule.scale)
            })
            .collect();
        out.insert(country.clone(), values);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rates::RULES;

    fn rule(name: &str) -> &'static Rule {
        RULES.iter().find(|r| r.name == name).unwrap()
    }

    fn rows(list: &[(&str, &[f64])]) -> Rows {
        list.iter()
            .map(|(c, v)| (c.to_string(), v.to_vec()))
            .collect()
    }

    #[test]
    fn missing_part_or_country_gives_nan_or_nothing() {
        let nan = f64::NAN;
        let mut den = HashMap::new();
        den.insert(
            "SP.POP.1564.TO".to_string(),
            rows(&[("AAA", &[100.0, nan])]),
        );
        let rate = rows(&[("AAA", &[10.0, 10.0]), ("BBB", &[1.0, 1.0])]);
        let r = rule("pct-ages-15-up");
        assert!(product(r, &rate, &den).is_empty());
        den.insert("SP.POP.65UP.TO".to_string(), rows(&[("AAA", &[50.0, 5.0])]));
        let out = product(r, &rate, &den);
        assert_eq!(out.len(), 1);
        assert_eq!(out["AAA"][0], 15.0);
        assert!(out["AAA"][1].is_nan());
    }

    #[test]
    fn overflow_of_f32_is_nan() {
        let mut den = HashMap::new();
        den.insert("SP.POP.TOTL".to_string(), rows(&[("AAA", &[1e300])]));
        let out = product(rule("per-capita-money"), &rows(&[("AAA", &[1e300])]), &den);
        assert!(out["AAA"][0].is_nan());
    }
}
