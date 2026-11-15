use pipeline::index;
use pipeline::wdi::{self, Stats};
use std::fs;
use std::path::{Path, PathBuf};

fn input() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pipeline-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Decodes a series file by hand, independent of the encoder.
fn decode(bytes: &[u8]) -> (Vec<String>, u16, u16, Vec<f32>) {
    assert_eq!(&bytes[0..4], b"MSR1");
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let (n, first, years) = (u16_at(4) as usize, u16_at(6), u16_at(8));
    assert_eq!(u16_at(10), 0);
    let codes = (0..n)
        .map(|i| String::from_utf8(bytes[12 + 3 * i..15 + 3 * i].to_vec()).unwrap())
        .collect();
    let data = (12 + 3 * n).div_ceil(4) * 4;
    assert_eq!(bytes.len(), data + 4 * n * years as usize);
    let values = bytes[data..]
        .chunks(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    (codes, first, years, values)
}

fn run_and_write(name: &str, min: usize) -> (PathBuf, Stats) {
    let (indicators, stats) = wdi::run(&input(), min).unwrap();
    let out = out_dir(name);
    index::write(&out, &indicators).unwrap();
    (out, stats)
}

fn read_series(out: &Path, slug: &str) -> (Vec<String>, u16, u16, Vec<f32>) {
    decode(&fs::read(out.join(format!("series/{slug}.bin"))).unwrap())
}

#[test]
fn each_rule_drops_its_indicator() {
    let (out, stats) = run_and_write("rules", 3);
    assert_eq!(
        stats,
        Stats {
            total: 11,
            no_metadata: 1,
            not_sum: 1,
            not_cc_by: 1,
            negative: 1,
            too_few_countries: 1,
            kept: 6,
        }
    );
    let mut files: Vec<String> = fs::read_dir(out.join("series"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "wdi-a-keep-totl.bin",
            "wdi-f-noparen.bin",
            "wdi-g-negagg.bin",
            "wdi-h-nested.bin",
            "wdi-j-oldneg.bin",
            "wdi-k-dots.bin"
        ]
    );
}

#[test]
fn aggregates_dropped_years_trimmed_entities_sorted() {
    let (out, _) = run_and_write("trim", 3);
    let (codes, first, years, values) = read_series(&out, "wdi-a-keep-totl");
    assert_eq!(codes, ["AAA", "BBB", "DDD"]);
    assert_eq!((first, years), (1947, 3));
    let nan = f32::NAN;
    let expected = [1.5, nan, 5.0, 2.0, 3.0, 4.0, nan, nan, 9.0];
    assert_eq!(values.len(), expected.len());
    for (got, want) in values.iter().zip(expected) {
        assert!(got.to_bits() == want.to_bits(), "{got} vs {want}");
    }
}

#[test]
fn years_before_1946_are_cut() {
    let (out, _) = run_and_write("cutoff", 3);
    let (codes, first, years, values) = read_series(&out, "wdi-f-noparen");
    assert_eq!(codes, ["AAA", "BBB", "CCC"]);
    assert_eq!((first, years), (1946, 1));
    assert_eq!(values, [1.0, 2.0, 3.0]);
}

#[test]
fn negative_aggregate_does_not_drop_the_indicator() {
    let (out, _) = run_and_write("negagg", 3);
    let (codes, first, years, values) = read_series(&out, "wdi-g-negagg");
    assert_eq!(codes, ["AAA", "BBB", "CCC", "DDD"]);
    assert_eq!((first, years), (1948, 1));
    assert_eq!(values, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn threshold_is_a_parameter() {
    let (_, strict) = run_and_write("strict", 4);
    assert_eq!(strict.kept, 1);
    let (_, loose) = run_and_write("loose", 1);
    assert_eq!(loose.kept, 7);
    assert_eq!(loose.too_few_countries, 0);
}

#[test]
fn index_titles_units_and_order() {
    let (out, _) = run_and_write("index", 3);
    let json = fs::read_to_string(out.join("index.json")).unwrap();
    let expected = concat!(
        r#"{"version":1,"series":["#,
        r#"{"slug":"wdi-g-negagg","title":"Widgets by region","unit":"count","source":"World Bank WDI","first":1948,"last":1948,"entities":4},"#,
        r#"{"slug":"wdi-a-keep-totl","title":"Widgets, total","unit":"million units","source":"World Bank WDI","first":1947,"last":1949,"entities":3},"#,
        r#"{"slug":"wdi-f-noparen","title":"Plain title","unit":"","source":"World Bank WDI","first":1946,"last":1946,"entities":3},"#,
        r#"{"slug":"wdi-h-nested","title":"Land (area) of things","unit":"sq. km","source":"World Bank WDI","first":1950,"last":1950,"entities":3},"#,
        r#"{"slug":"wdi-j-oldneg","title":"Early negatives","unit":"units","source":"World Bank WDI","first":1946,"last":1946,"entities":3},"#,
        r#"{"slug":"wdi-k-dots","title":"Sparse widgets","unit":"","source":"World Bank WDI","first":1947,"last":1947,"entities":3}"#,
        r#"]}"#
    );
    assert_eq!(json, expected);
}

#[test]
fn negatives_before_1946_are_ignored() {
    let (out, _) = run_and_write("oldneg", 3);
    let (codes, first, years, values) = read_series(&out, "wdi-j-oldneg");
    assert_eq!(codes, ["AAA", "BBB", "CCC"]);
    assert_eq!((first, years), (1946, 1));
    assert_eq!(values, [1.0, 2.0, 3.0]);
}

#[test]
fn unusable_cells_and_odd_codes_do_not_abort_the_run() {
    let (out, _) = run_and_write("dots", 3);
    let (codes, first, years, values) = read_series(&out, "wdi-k-dots");
    assert_eq!(codes, ["AAA", "BBB", "CCC"]);
    assert_eq!((first, years), (1947, 1));
    assert_eq!(values, [1.0, 2.0, 3.0]);
}
