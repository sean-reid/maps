mod common;

use common::{out_dir, read_series};
use pipeline::build::{self, Inputs};
use pipeline::{fao, index, wdi};
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fao_dir() -> PathBuf {
    fixtures().join("fao")
}

#[test]
fn build_with_one_source_writes_that_source_unchanged() {
    let wdi = fixtures();
    let (indicators, reports) = build::run(
        &Inputs {
            wdi: &wdi,
            fao: None,
        },
        3,
    )
    .unwrap();
    assert_eq!(reports.len(), 1);
    let out = out_dir("build-wdi");
    index::write(&out, &indicators).unwrap();
    let (codes, first, years, values) = read_series(&out, "wdi-f-noparen");
    assert_eq!(codes, ["AAA", "BBB", "CCC"]);
    assert_eq!((first, years), (1946, 1));
    assert_eq!(values, [1.0, 2.0, 3.0]);
    assert_eq!(indicators.len(), 6);
}

#[test]
fn both_sources_merge_into_one_index_and_series_directory() {
    let wdi = fixtures();
    let inputs = Inputs {
        wdi: &wdi,
        fao: Some(&fao_dir()),
    };
    let (indicators, _) = build::run(&inputs, 3).unwrap();
    let out = out_dir("fao-merged");
    index::write(&out, &indicators).unwrap();
    let files = series_files(&out);
    assert_eq!(files.len(), 13);
    assert!(files.contains(&"wdi-a-keep-totl.bin".to_string()));
    assert!(files.contains(&"fao-qcl-15-5510.bin".to_string()));
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("index.json")).unwrap()).unwrap();
    let sources: Vec<&str> = json["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["source"].as_str().unwrap())
        .collect();
    assert_eq!(sources.iter().filter(|s| **s == "FAOSTAT").count(), 7);
    assert_eq!(
        sources.iter().filter(|s| **s == "World Bank WDI").count(),
        6
    );
}

#[test]
fn a_slug_shared_between_sources_is_an_error() {
    let wdi = fixtures();
    let (mut fao_side, _) = fao::run(&fao_dir(), 3).unwrap();
    let (wdi_side, _) = wdi::run(&wdi, 3).unwrap();
    fao_side[0].code = wdi_side[0].slug();
    let err = build::merge(wdi_side, fao_side).unwrap_err().to_string();
    assert!(err.contains("is used by both"), "{err}");
}

fn series_files(out: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(out.join("series"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    files
}
