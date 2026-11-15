mod common;

use common::{out_dir, read_series};
use pipeline::build::{self, Inputs};
use pipeline::index;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn build_with_one_source_writes_that_source_unchanged() {
    let wdi = fixtures();
    let (indicators, reports) = build::run(&Inputs { wdi: &wdi }, 3).unwrap();
    assert_eq!(reports.len(), 1);
    let out = out_dir("build-wdi");
    index::write(&out, &indicators).unwrap();
    let (codes, first, years, values) = read_series(&out, "wdi-f-noparen");
    assert_eq!(codes, ["AAA", "BBB", "CCC"]);
    assert_eq!((first, years), (1946, 1));
    assert_eq!(values, [1.0, 2.0, 3.0]);
    assert_eq!(indicators.len(), 6);
}
