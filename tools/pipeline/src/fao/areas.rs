use crate::Result;
use std::collections::HashMap;

const TABLE: &str = include_str!("../../data/m49.csv");
/// FAO area codes from here up are regions, income groups, and other aggregates.
const FIRST_AGGREGATE: u32 = 5000;
/// "China" (351) adds mainland, Hong Kong, Macao, and Taiwan, which have their own rows.
const COMBINED: [u32; 1] = [351];

#[derive(Debug, PartialEq)]
pub enum Area {
    Country(String),
    Aggregate,
    /// A real FAO area with no M49 code on the map.
    Unmapped,
}

pub struct Areas(HashMap<String, String>);

impl Areas {
    pub fn embedded() -> Result<Self> {
        Self::parse(TABLE)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines();
        if lines.next() != Some("m49,iso3") {
            return Err("m49 table needs the header m49,iso3".into());
        }
        let mut map = HashMap::new();
        for line in lines {
            let (m49, iso) = line.split_once(',').ok_or("bad m49 row")?;
            if m49.len() != 3 || iso.len() != 3 {
                return Err(format!("bad m49 row {line:?}").into());
            }
            if map.insert(m49.to_string(), iso.to_string()).is_some() {
                return Err(format!("m49 {m49} appears twice").into());
            }
        }
        Ok(Areas(map))
    }

    pub fn classify(&self, area_code: &str, m49: &str) -> Area {
        let code: u32 = area_code.trim().parse().unwrap_or(u32::MAX);
        if code >= FIRST_AGGREGATE || COMBINED.contains(&code) {
            return Area::Aggregate;
        }
        match self.0.get(m49.trim().trim_start_matches('\'')) {
            Some(iso) => Area::Country(iso.clone()),
            None => Area::Unmapped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_maps_known_codes() {
        let areas = Areas::embedded().unwrap();
        assert_eq!(areas.classify("2", "'004"), Area::Country("AFG".into()));
        assert_eq!(areas.classify("41", "'156"), Area::Country("CHN".into()));
        assert_eq!(areas.classify("107", "'384"), Area::Country("CIV".into()));
    }

    #[test]
    fn aggregates_and_unknown_codes_are_not_countries() {
        let areas = Areas::embedded().unwrap();
        assert_eq!(areas.classify("5000", "'001"), Area::Aggregate);
        assert_eq!(areas.classify("351", "'159"), Area::Aggregate);
        assert_eq!(areas.classify("228", "'810"), Area::Unmapped);
        assert_eq!(areas.classify("420", "'202"), Area::Unmapped);
    }

    #[test]
    fn rejects_a_malformed_table() {
        assert!(Areas::parse("a,b\n004,AFG\n").is_err());
        assert!(Areas::parse("m49,iso3\n04,AFG\n").is_err());
        assert!(Areas::parse("m49,iso3\n004,AFG\n004,ALB\n").is_err());
    }
}
