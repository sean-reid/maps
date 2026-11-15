use crate::format::{encode, Series};
use crate::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub const SOURCE: &str = "World Bank WDI";

pub struct Indicator {
    pub code: String,
    pub title: String,
    pub unit: String,
    pub series: Series,
}

#[derive(Serialize)]
pub struct Entry {
    pub slug: String,
    pub title: String,
    pub unit: String,
    pub source: String,
    pub first: u16,
    pub last: u16,
    pub entities: usize,
}

#[derive(Serialize)]
pub struct Index {
    pub version: u8,
    pub series: Vec<Entry>,
}

pub fn slug(code: &str) -> String {
    format!("wdi-{}", code.to_lowercase().replace('.', "-"))
}

pub fn build(indicators: &[Indicator]) -> Index {
    let mut series: Vec<Entry> = indicators
        .iter()
        .map(|i| Entry {
            slug: slug(&i.code),
            title: i.title.clone(),
            unit: i.unit.clone(),
            source: SOURCE.to_string(),
            first: i.series.first_year,
            last: i.series.last_year(),
            entities: i.series.codes.len(),
        })
        .collect();
    series.sort_by(|a, b| {
        b.entities
            .cmp(&a.entities)
            .then_with(|| a.slug.cmp(&b.slug))
    });
    Index { version: 1, series }
}

/// Writes `series/<slug>.bin` and `index.json`; returns total bytes under `series/`.
pub fn write(out: &Path, indicators: &[Indicator]) -> Result<u64> {
    let mut files = Vec::with_capacity(indicators.len());
    let mut bytes = 0;
    for i in indicators {
        let data = encode(&i.series).map_err(|e| format!("{}: {e}", i.code))?;
        bytes += data.len() as u64;
        files.push((format!("{}.bin", slug(&i.code)), data));
    }
    let dir = out.join("series");
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    for (name, data) in files {
        fs::write(dir.join(name), data)?;
    }
    fs::write(
        out.join("index.json"),
        serde_json::to_vec(&build(indicators))?,
    )?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ind(code: &str, n: usize) -> Indicator {
        Indicator {
            code: code.into(),
            title: "t".into(),
            unit: String::new(),
            series: Series {
                codes: (0..n).map(|i| format!("A{:02}", i)).collect(),
                first_year: 2000,
                years: 2,
                values: vec![1.0; n * 2],
            },
        }
    }

    #[test]
    fn slug_lowercases_and_hyphenates() {
        assert_eq!(slug("SP.POP.TOTL"), "wdi-sp-pop-totl");
        assert_eq!(slug("AG.LND.TOTL.K2"), "wdi-ag-lnd-totl-k2");
    }

    #[test]
    fn sorted_by_entities_desc_then_slug() {
        let idx = build(&[ind("B.B", 3), ind("A.A", 3), ind("C.C", 5), ind("D.D", 1)]);
        let slugs: Vec<&str> = idx.series.iter().map(|e| e.slug.as_str()).collect();
        assert_eq!(slugs, ["wdi-c-c", "wdi-a-a", "wdi-b-b", "wdi-d-d"]);
    }

    #[test]
    fn json_shape() {
        let json = serde_json::to_string(&build(&[ind("X.Y", 2)])).unwrap();
        assert_eq!(
            json,
            r#"{"version":1,"series":[{"slug":"wdi-x-y","title":"t","unit":"","source":"World Bank WDI","first":2000,"last":2001,"entities":2}]}"#
        );
    }

    #[test]
    fn rerun_removes_stale_series() {
        let out = std::env::temp_dir().join(format!("pipeline-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(out.join("series")).unwrap();
        fs::write(out.join("series/old.bin"), b"x").unwrap();
        write(&out, &[ind("X.Y", 2)]).unwrap();
        let mut names: Vec<_> = fs::read_dir(out.join("series"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["wdi-x-y.bin"]);
        fs::remove_dir_all(&out).unwrap();
    }
}
