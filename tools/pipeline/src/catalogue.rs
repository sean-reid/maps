//! Rules for combining native and derived series into one catalogue.

use crate::index::Indicator;
use crate::rates::Plan;
use crate::Result;
use std::collections::{HashMap, HashSet};

/// Lowercases, collapses spaces, and spells "USD" as "US$".
pub fn norm_unit(unit: &str) -> String {
    let unit = unit.to_lowercase().replace("usd", "us$");
    // WDI writes both "BoP, current US$" and "current US$, BoP".
    let mut parts: Vec<String> = unit
        .split(',')
        .map(|part| part.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    parts.sort();
    parts.join(", ")
}

/// A native total with the same title wins when its unit matches or is not stated.
pub fn is_native(native: &HashSet<(String, String)>, title: &str, unit: &str) -> bool {
    let key = |unit: &str| (title.to_string(), norm_unit(unit));
    native.contains(&key(unit)) || native.contains(&key(""))
}

pub struct Derived<'a> {
    pub indicator: Indicator,
    pub plan: &'a Plan,
    tagged: bool,
}

impl<'a> Derived<'a> {
    pub fn new(indicator: Indicator, plan: &'a Plan) -> Self {
        Derived {
            indicator,
            plan,
            tagged: false,
        }
    }
}

fn collisions(items: &[Derived]) -> Vec<Vec<usize>> {
    let mut groups: HashMap<(&str, String), Vec<usize>> = HashMap::new();
    for (at, d) in items.iter().enumerate() {
        let key = (d.indicator.title.as_str(), norm_unit(&d.indicator.unit));
        groups.entry(key).or_default().push(at);
    }
    let mut found: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() > 1).collect();
    found.sort();
    found
}

/// Gives colliding series their rule's tag when every rule has a different one; otherwise keeps the
/// series with the most countries. Returns the series, how many were retitled, and how many dropped.
pub fn resolve_twins(mut items: Vec<Derived>) -> (Vec<Derived>, usize, usize) {
    let (mut retitled, mut dropped) = (0, 0);
    let mut gone: HashSet<usize> = HashSet::new();
    while let Some(group) = collisions(&items).into_iter().next() {
        let tags: HashSet<&str> = group.iter().map(|&at| items[at].plan.rule.tag).collect();
        let distinct = tags.len() == group.len() && !tags.contains("");
        if distinct && group.iter().all(|&at| !items[at].tagged) {
            for &at in &group {
                items[at].indicator.title = items[at].plan.tagged_title();
                items[at].tagged = true;
            }
            retitled += group.len();
            continue;
        }
        let best = *group
            .iter()
            .max_by(|&&a, &&b| {
                let (x, y) = (&items[a].indicator, &items[b].indicator);
                (x.series.codes.len(), y.slug()).cmp(&(y.series.codes.len(), x.slug()))
            })
            .unwrap();
        gone.extend(group.iter().copied().filter(|&at| at != best));
        dropped += group.len() - 1;
        let keep: Vec<Derived> = items
            .into_iter()
            .enumerate()
            .filter(|(at, _)| !gone.contains(at))
            .map(|(_, d)| d)
            .collect();
        items = keep;
        gone.clear();
    }
    (items, retitled, dropped)
}

/// Native and derived series must not share a slug; the error names both codes.
pub fn check_slugs(all: &[Indicator]) -> Result<()> {
    let mut seen: HashMap<String, &str> = HashMap::new();
    for i in all {
        if let Some(other) = seen.insert(i.slug(), &i.code) {
            return Err(format!("slug {} is used by both {other} and {}", i.slug(), i.code).into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_unit_is_the_same_whatever_order_its_parts_come_in() {
        assert_eq!(
            norm_unit("current US$, BoP"),
            norm_unit("BoP,  current USD")
        );
        assert_ne!(norm_unit("current US$"), norm_unit("constant 2015 US$"));
    }

    use super::*;
    use crate::format::Series;
    use crate::index::Source;
    use crate::rates::plan;

    fn item<'a>(code: &str, countries: usize, plan: &'a Plan) -> Derived<'a> {
        let indicator = Indicator {
            source: Source::Wdi,
            code: code.into(),
            title: plan.title.clone(),
            unit: plan.unit.clone(),
            series: Series {
                codes: (0..countries).map(|i| format!("A{i:02}")).collect(),
                first_year: 2000,
                years: 1,
                values: vec![1.0; countries],
            },
            derived: Some("x".into()),
        };
        Derived::new(indicator, plan)
    }

    #[test]
    fn units_compare_after_normalising_spelling() {
        assert_eq!(norm_unit(" Current  USD "), norm_unit("current US$"));
        let native: HashSet<(String, String)> =
            [("GDP".to_string(), norm_unit("current US$"))].into();
        assert!(is_native(&native, "GDP", "Current USD"));
        assert!(!is_native(&native, "GDP", "sq. km"));
    }

    #[test]
    fn same_title_with_distinct_tags_is_retitled_and_without_is_merged() {
        let a = plan("Services (% of service exports, BoP)", "").unwrap();
        let b = plan("Services (% of service imports, BoP)", "").unwrap();
        let (kept, retitled, dropped) = resolve_twins(vec![item("A", 5, &a), item("B", 5, &b)]);
        let titles: Vec<&str> = kept.iter().map(|d| d.indicator.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Services, share of service exports",
                "Services, share of service imports"
            ]
        );
        assert_eq!((retitled, dropped), (2, 0));

        let c = plan("Spending (per 1,000 people)", "").unwrap();
        let d = plan("Spending (per 100 people)", "").unwrap();
        let (kept, retitled, dropped) = resolve_twins(vec![item("C", 3, &c), item("D", 9, &d)]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].indicator.code, "D");
        assert_eq!((retitled, dropped), (0, 1));
    }

    #[test]
    fn a_shared_slug_is_an_error_naming_both_codes() {
        let p = plan("Spending (per 1,000 people)", "").unwrap();
        let mut native = item("X.Y-TOTAL", 1, &p).indicator;
        native.derived = None;
        let derived = item("X.Y", 1, &p).indicator;
        let err = check_slugs(&[native, derived]).unwrap_err().to_string();
        assert!(
            err.contains("wdi-x-y-total") && err.contains("X.Y-TOTAL") && err.contains("X.Y"),
            "{err}"
        );
    }
}
