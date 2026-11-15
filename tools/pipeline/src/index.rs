use crate::format::{encode, Series};
use crate::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Wdi,
    Fao,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Wdi => "World Bank WDI",
            Source::Fao => "FAOSTAT",
        }
    }
}

#[derive(Debug)]
pub struct Indicator {
    pub source: Source,
    /// The source's own code; for FAOSTAT it is already the slug.
    pub code: String,
    pub title: String,
    pub unit: String,
    pub series: Series,
    /// Set when the series is a rate multiplied by a denominator.
    pub derived: Option<String>,
}

impl Indicator {
    pub fn slug(&self) -> String {
        match (self.source, &self.derived) {
            (Source::Fao, _) => self.code.clone(),
            (Source::Wdi, Some(_)) => format!("{}-total", slug(&self.code)),
            (Source::Wdi, None) => slug(&self.code),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived: Option<String>,
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
            slug: i.slug(),
            title: i.title.clone(),
            unit: i.unit.clone(),
            source: i.source.name().to_string(),
            first: i.series.first_year,
            last: i.series.last_year(),
            entities: i.series.codes.len(),
            derived: i.derived.clone(),
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
        files.push((format!("{}.bin", i.slug()), data));
    }
    if out.as_os_str().is_empty() {
        return Err("--out is empty".into());
    }
    // Everything is staged first, so a failed write leaves the previous output whole.
    let staged = out.join("series.tmp");
    if staged.exists() {
        fs::remove_dir_all(&staged)?;
    }
    fs::create_dir_all(&staged)?;
    let index = out.join("index.json.tmp");
    let stage = || -> Result<()> {
        for (name, data) in &files {
            fs::write(staged.join(name), data)?;
        }
        fs::write(&index, serde_json::to_vec(&build(indicators))?)?;
        Ok(())
    };
    if let Err(e) = stage() {
        let _ = fs::remove_dir_all(&staged);
        let _ = fs::remove_file(&index);
        return Err(e);
    }
    let dir = out.join("series");
    fs::create_dir_all(&dir)?;
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "bin") {
            fs::remove_file(path)?;
        }
    }
    for (name, _) in &files {
        fs::rename(staged.join(name), dir.join(name))?;
    }
    fs::remove_dir_all(&staged)?;
    fs::rename(&index, out.join("index.json"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ind(code: &str, n: usize) -> Indicator {
        Indicator {
            source: Source::Wdi,
            code: code.into(),
            title: "t".into(),
            unit: String::new(),
            derived: None,
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
    fn derived_series_get_a_total_slug_and_a_note() {
        let mut i = ind("X.Y", 2);
        i.derived = Some("share of population times population".into());
        let json = serde_json::to_string(&build(&[i])).unwrap();
        assert_eq!(
            json,
            r#"{"version":1,"series":[{"slug":"wdi-x-y-total","title":"t","unit":"","source":"World Bank WDI","first":2000,"last":2001,"entities":2,"derived":"share of population times population"}]}"#
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

    #[test]
    fn only_series_files_are_replaced_and_a_failed_run_changes_nothing() {
        let out = std::env::temp_dir().join(format!("pipeline-keep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(out.join("series")).unwrap();
        fs::write(out.join("series/notes.txt"), b"mine").unwrap();
        write(&out, &[ind("X.Y", 2)]).unwrap();
        let before = fs::read(out.join("index.json")).unwrap();

        let mut bad = ind("Z.Z", 1);
        bad.series.codes = vec!["TOOLONG".into()];
        assert!(write(&out, &[bad]).is_err());
        assert!(write(Path::new(""), &[ind("X.Y", 2)]).is_err());

        assert_eq!(fs::read(out.join("index.json")).unwrap(), before);
        assert_eq!(fs::read(out.join("series/notes.txt")).unwrap(), b"mine");
        assert!(out.join("series/wdi-x-y.bin").exists());
        assert!(!out.join("series.tmp").exists());
        assert!(!out.join("index.json.tmp").exists());
        fs::remove_dir_all(&out).unwrap();
    }
}
