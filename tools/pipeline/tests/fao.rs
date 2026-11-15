mod common;

use common::{out_dir, read_series};
use pipeline::fao::{self, Stats};
use pipeline::index;
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fao_dir() -> PathBuf {
    fixtures().join("fao")
}

fn run_and_write(name: &str) -> (PathBuf, Stats) {
    let (indicators, stats) = fao::run(&fao_dir(), 3).unwrap();
    let out = out_dir(name);
    index::write(&out, &indicators).unwrap();
    (out, stats)
}

fn series_files(out: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(out.join("series"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    files
}

#[test]
fn only_summable_totals_with_enough_countries_are_written() {
    let (out, _) = run_and_write("fao-files");
    assert_eq!(
        series_files(&out),
        [
            "fao-fo-1638-5622.bin",
            "fao-fo-1651-5616.bin",
            "fao-fo-1866-5616.bin",
            "fao-gt-5060-723113.bin",
            "fao-qcl-15-5510.bin",
            "fao-qcl-866-5112.bin",
            "fao-rl-6600-5110.bin"
        ]
    );
}

#[test]
fn areas_map_through_m49_and_aggregates_and_the_combined_china_row_are_dropped() {
    let (out, _) = run_and_write("fao-wheat");
    let (codes, first, years, values) = read_series(&out, "fao-qcl-15-5510");
    assert_eq!(codes, ["AFG", "ALB", "CHN", "DZA", "TWN"]);
    assert_eq!((first, years), (1990, 2));
    let nan = f32::NAN;
    let expected = [
        100.0, 110.0, 200.0, nan, 1000.0, 1100.0, 300.0, 310.0, 7.0, 8.0,
    ];
    assert_eq!(values.len(), expected.len());
    for (got, want) in values.iter().zip(expected) {
        assert!(got.to_bits() == want.to_bits(), "{got} vs {want}");
    }
}

#[test]
fn historical_entities_are_reported_with_their_years_and_series() {
    let (_, stats) = run_and_write("fao-historic");
    let ussr = &stats.unmapped[&("228".to_string(), "USSR".to_string())];
    assert_eq!((ussr.first, ussr.last, ussr.series), (1990, 1991, 1));
    let africa = &stats.unmapped[&("420".to_string(), "Sub-Saharan Africa".to_string())];
    assert_eq!((africa.first, africa.last, africa.series), (1990, 1990, 1));
    assert_eq!(stats.unmapped.len(), 2);
    assert!(stats.aggregates.contains("World"));
    assert!(stats.aggregates.contains("China"));
}

#[test]
fn units_scale_to_their_base() {
    let (out, _) = run_and_write("fao-scale");
    let (codes, _, _, cattle) = read_series(&out, "fao-qcl-866-5112");
    assert_eq!(codes, ["AFG", "ALB", "DZA"]);
    assert_eq!(cattle, [1500.0, 2000.0, 250.0]);
    let (_, _, _, land) = read_series(&out, "fao-rl-6600-5110");
    assert_eq!(land, [65286000.0, 2740000.0, 238174000.0]);
    let (_, _, _, dollars) = read_series(&out, "fao-fo-1638-5622");
    assert_eq!(dollars, [1981000.0, 1787000.0, 5000.0]);
}

#[test]
fn years_before_1946_and_forecast_rows_are_cut_and_the_lower_source_code_wins() {
    let (out, _) = run_and_write("fao-source");
    let (codes, first, years, values) = read_series(&out, "fao-gt-5060-723113");
    assert_eq!(codes, ["AFG", "ALB", "DZA"]);
    assert_eq!((first, years), (1990, 1));
    assert_eq!(values, [10.0, 20.0, 30.0]);
}

#[test]
fn every_rule_has_its_count() {
    let (_, stats) = run_and_write("fao-stats");
    let qcl = &stats.domains["QCL"];
    assert_eq!((qcl.kept, qcl.negative, qcl.too_few_countries), (2, 1, 2));
    let rl = &stats.domains["RL"];
    assert_eq!((rl.kept, rl.items.len(), rl.elements.len()), (1, 1, 1));
    let share = &rl.pairs[&("7208".into(), "Share in Land area".into(), "%".into())];
    assert!(!share.kept && share.series == 1);
    let yield_pair = &stats.domains["QCL"].pairs[&("5412".into(), "Yield".into(), "kg/ha".into())];
    assert!(!yield_pair.kept);
    assert_eq!(stats.kept(), 7);
}

#[test]
fn index_titles_units_and_order() {
    let (out, _) = run_and_write("fao-index");
    let json = fs::read_to_string(out.join("index.json")).unwrap();
    let entry = |slug: &str, title: &str, unit: &str, first: u16, last: u16, n: usize| {
        format!(
            r#"{{"slug":"{slug}","title":"{title}","unit":"{unit}","source":"FAOSTAT","first":{first},"last":{last},"entities":{n}}}"#
        )
    };
    let expected = [
        entry("fao-qcl-15-5510", "Wheat, production", "t", 1990, 1991, 5),
        entry(
            "fao-fo-1638-5622",
            "Sawdust, import value",
            "US$",
            2000,
            2000,
            3,
        ),
        entry(
            "fao-fo-1651-5616",
            "Industrial roundwood, coniferous (export/import), import quantity",
            "m3",
            2000,
            2000,
            3,
        ),
        entry(
            "fao-fo-1866-5616",
            "Industrial roundwood, coniferous, import quantity",
            "m3",
            2000,
            2000,
            3,
        ),
        entry(
            "fao-gt-5060-723113",
            "Rice Cultivation, emissions (CO2eq) (AR5)",
            "kt",
            1990,
            1990,
            3,
        ),
        entry("fao-qcl-866-5112", "Cattle, stocks", "head", 1990, 1990, 3),
        entry(
            "fao-rl-6600-5110",
            "Country area, area",
            "ha",
            1990,
            1990,
            3,
        ),
    ]
    .join(",");
    assert_eq!(json, format!(r#"{{"version":1,"series":[{expected}]}}"#));
}

#[test]
fn a_missing_directory_or_file_set_is_an_error() {
    assert!(fao::run(&fixtures().join("nope"), 3).is_err());
    assert!(fao::run(&fixtures().join("rates"), 3).is_err());
}

const HEAD: &str = "Area Code,Area Code (M49),Area,Item Code,Item,Element Code,Element,Year Code,Year,Unit,Value,Flag,Note";
const HEAD_SOURCE: &str = "Area Code,Area Code (M49),Area,Item Code,Item,Element Code,Element,Year Code,Year,Source Code,Source,Unit,Value,Flag,Note";

fn row(area: (&str, &str, &str), item: &str, year: &str, unit: &str, value: &str) -> String {
    let (code, m49, name) = area;
    format!(
        "\"{code}\",\"'{m49}\",\"{name}\",\"{item}\",\"Wheat\",\"5510\",\"Production\",\"{year}\",\"{year}\",\"{unit}\",\"{value}\",\"A\",\n"
    )
}

fn row_source(area: (&str, &str, &str), source: &str, value: &str) -> String {
    let (code, m49, name) = area;
    format!(
        "\"{code}\",\"'{m49}\",\"{name}\",\"15\",\"Wheat\",\"5510\",\"Production\",\"1990\",\"1990\",\"{source}\",\"S\",\"t\",\"{value}\",\"A\",\n"
    )
}

const AFG: (&str, &str, &str) = ("2", "004", "Afghanistan");
const ALB: (&str, &str, &str) = ("3", "008", "Albania");
const DZA: (&str, &str, &str) = ("4", "012", "Algeria");

fn domain_dir(name: &str, file: &str, text: &str) -> PathBuf {
    let dir = out_dir(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(file), text).unwrap();
    dir
}

#[test]
fn two_areas_on_one_iso_fail_naming_both() {
    let other = ("999", "004", "Afghanistan again");
    let text = format!(
        "{HEAD}\n{}{}",
        row(AFG, "15", "1990", "t", "1"),
        row(other, "15", "1990", "t", "2")
    );
    let dir = domain_dir("fao-twin-iso", "QCL.csv", &text);
    let err = fao::run(&dir, 1).unwrap_err().to_string();
    assert!(
        err.contains("2 (Afghanistan)")
            && err.contains("999 (Afghanistan again)")
            && err.contains("AFG"),
        "{err}"
    );
}

#[test]
fn a_pair_with_two_base_units_becomes_two_series() {
    let mut text = format!("{HEAD}\n");
    for (area, t, number) in [(AFG, "1", "2"), (ALB, "3", "4"), (DZA, "5", "6")] {
        text += &row(area, "1062", "1990", "t", t);
        text += &row(area, "1062", "1990", "1000 No", number);
    }
    let dir = domain_dir("fao-units", "QCL.csv", &text);
    let (indicators, stats) = fao::run(&dir, 3).unwrap();
    let out = out_dir("fao-units-out");
    index::write(&out, &indicators).unwrap();
    assert_eq!(
        series_files(&out),
        ["fao-qcl-1062-5510-number.bin", "fao-qcl-1062-5510.bin"]
    );
    assert_eq!(read_series(&out, "fao-qcl-1062-5510").3, [1.0, 3.0, 5.0]);
    assert_eq!(
        read_series(&out, "fao-qcl-1062-5510-number").3,
        [2000.0, 4000.0, 6000.0]
    );
    assert_eq!(stats.domains["QCL"].mixed_units, 1);
}

#[test]
fn a_missing_or_unparseable_source_code_loses_to_a_real_one() {
    let text = format!(
        "{HEAD_SOURCE}\n{}{}{}{}",
        row_source(AFG, "", "99"),
        row_source(AFG, "3051", "5"),
        row_source(ALB, "x", "7"),
        row_source(DZA, "3050", "30"),
    );
    let dir = domain_dir("fao-rank", "GT.csv", &text);
    let (indicators, _) = fao::run(&dir, 3).unwrap();
    let out = out_dir("fao-rank-out");
    index::write(&out, &indicators).unwrap();
    assert_eq!(read_series(&out, "fao-gt-15-5510").3, [5.0, 7.0, 30.0]);
}

#[test]
fn two_rows_for_one_cell_are_an_error_without_a_source_ranking_them() {
    let text = format!(
        "{HEAD}\n{}{}",
        row(AFG, "15", "1990", "t", "1"),
        row(AFG, "15", "1990", "t", "2")
    );
    let dir = domain_dir("fao-dup", "QCL.csv", &text);
    let err = fao::run(&dir, 1).unwrap_err().to_string();
    assert!(
        err.contains("area 2 (Afghanistan)")
            && err.contains("item 15")
            && err.contains("element 5510")
            && err.contains("year 1990"),
        "{err}"
    );
    let tied = format!(
        "{HEAD_SOURCE}\n{}{}",
        row_source(AFG, "3050", "1"),
        row_source(AFG, "3050", "2")
    );
    let dir = domain_dir("fao-tie", "GT.csv", &tied);
    assert!(fao::run(&dir, 1).is_err());
}

#[test]
fn years_that_are_not_a_single_year_in_range_are_counted_and_skipped() {
    let mut text = format!("{HEAD}\n");
    for area in [AFG, ALB, DZA] {
        text += &row(area, "15", "1990", "t", "1");
    }
    for year in ["2010-2012", "1945", "2101", "65535", "abc"] {
        text += &row(AFG, "15", year, "t", "9");
    }
    let dir = domain_dir("fao-years", "QCL.csv", &text);
    let (indicators, stats) = fao::run(&dir, 3).unwrap();
    assert_eq!(stats.skipped_years, 5);
    assert!(stats
        .to_string()
        .contains("rows with no usable year skipped: 5"));
    let out = out_dir("fao-years-out");
    index::write(&out, &indicators).unwrap();
    let (_, first, years, values) = read_series(&out, "fao-qcl-15-5510");
    assert_eq!((first, years), (1990, 1));
    assert_eq!(values, [1.0, 1.0, 1.0]);
}
