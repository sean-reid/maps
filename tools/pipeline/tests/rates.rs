mod common;

use common::{out_dir, read_series};
use pipeline::index;
use pipeline::wdi::{self, Stats};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const NAN: f32 = f32::NAN;

fn run_and_write(name: &str) -> (PathBuf, Stats) {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rates");
    let (indicators, stats) = wdi::run(&input, 3).unwrap();
    let out = out_dir(name);
    index::write(&out, &indicators).unwrap();
    (out, stats)
}

fn assert_series(out: &Path, slug: &str, want: [[f32; 3]; 4]) {
    let (codes, first, years, values) = read_series(out, slug);
    assert_eq!(codes, ["AAA", "BBB", "CCC", "DDD"], "{slug}");
    assert_eq!((first, years), (2019, 3), "{slug}");
    let want: Vec<f32> = want.concat();
    assert_eq!(values.len(), want.len(), "{slug}");
    for (got, want) in values.iter().zip(&want) {
        assert!(got.to_bits() == want.to_bits(), "{slug}: {got} vs {want}");
    }
}

fn entries(out: &Path) -> Vec<Value> {
    let json: Value = serde_json::from_slice(&fs::read(out.join("index.json")).unwrap()).unwrap();
    json["series"].as_array().unwrap().clone()
}

fn entry(out: &Path, slug: &str) -> Value {
    let all = entries(out);
    all.iter()
        .find(|e| e["slug"] == slug)
        .unwrap_or_else(|| panic!("no entry {slug}"))
        .clone()
}

#[test]
fn counts_and_files() {
    let (out, stats) = run_and_write("rates-counts");
    assert_eq!(
        (
            stats.total,
            stats.kept,
            stats.derived,
            stats.no_rule,
            stats.not_cc_by
        ),
        (40, 9, 15, 8, 3)
    );
    assert_eq!(
        (
            stats.negative,
            stats.too_few_countries,
            stats.duplicate,
            stats.retitled,
            stats.merged
        ),
        (1, 1, 2, 2, 1)
    );
    let mut files: Vec<String> = fs::read_dir(out.join("series"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    let derived = files.iter().filter(|f| f.ends_with("-total.bin")).count();
    assert_eq!((files.len(), derived), (24, 15));
}

#[test]
fn share_and_per_n_rules_multiply_by_population() {
    let (out, _) = run_and_write("rates-population");
    let cases: [(&str, [[f32; 3]; 4]); 5] = [
        (
            "wdi-it-net-user-zs",
            [
                [100.0, 400.0, 1200.0],
                [100.0, 200.0, 400.0],
                [NAN, 0.0, 50.0],
                [2.0, 4.0, 8.0],
            ],
        ),
        (
            "wdi-sh-med-beds-zs",
            [
                [2.0, 4.0, 8.0],
                [1.0, 2.0, 4.0],
                [NAN, 1.0, 2.0],
                [1.0, 2.0, 4.0],
            ],
        ),
        (
            "wdi-it-cel-sets-p2",
            [
                [500.0, 1000.0, 2000.0],
                [200.0, 400.0, 800.0],
                [NAN, 200.0, 400.0],
                [1.0, 2.0, 4.0],
            ],
        ),
        (
            "wdi-sh-sta-traf-p5",
            [
                [1.0, 2.0, 4.0],
                [1.0, 2.0, 4.0],
                [NAN, 1.0, 2.0],
                [1.0, 2.0, 4.0],
            ],
        ),
        (
            "wdi-sp-pop-scie-rd-p6",
            [
                [1.0, 2.0, 4.0],
                [1.0, 2.0, 4.0],
                [NAN, 1.0, 2.0],
                [1.0, 2.0, 4.0],
            ],
        ),
    ];
    for (slug, want) in cases {
        assert_series(&out, &format!("{slug}-total"), want);
    }
}

#[test]
fn a_missing_denominator_year_leaves_that_year_empty() {
    let (out, _) = run_and_write("rates-missing");
    let (_, _, _, values) = read_series(&out, "wdi-it-net-user-zs-total");
    assert!(values[6].is_nan());
    assert_eq!(values[7], 0.0);
}

#[test]
fn money_area_labor_and_age_band_rules() {
    let (out, _) = run_and_write("rates-other");
    assert_series(
        &out,
        "wdi-gb-xpd-rsdv-gd-zs-total",
        [
            [10000.0, 20000.0, 40000.0],
            [10000.0; 3],
            [5000.0, 10000.0, 20000.0],
            [1000.0, 2000.0, 4000.0],
        ],
    );
    assert_series(
        &out,
        "wdi-ag-lnd-prct-zs-total",
        [[50.0; 3], [50.0; 3], [50.0; 3], [10.0; 3]],
    );
    assert_series(
        &out,
        "wdi-sl-uem-totl-zs-total",
        [
            [40.0, 80.0, 160.0],
            [40.0, 80.0, 160.0],
            [5.0, 10.0, 20.0],
            [1.0, 2.0, 4.0],
        ],
    );
    assert_series(
        &out,
        "wdi-fx-own-totl-zs-total",
        [
            [350.0, 700.0, 1400.0],
            [60.0, 120.0, 240.0],
            [20.0, 40.0, 80.0],
            [3.0, 6.0, 12.0],
        ],
    );
}

#[test]
fn per_capita_rules_multiply_and_rename() {
    let (out, _) = run_and_write("rates-capita");
    assert_series(
        &out,
        "wdi-sh-xpd-chex-pc-cd-total",
        [
            [1000.0, 2000.0, 4000.0],
            [2000.0, 4000.0, 8000.0],
            [NAN, 10000.0, 20000.0],
            [10000.0, 20000.0, 40000.0],
        ],
    );
    assert_series(
        &out,
        "wdi-eg-use-elec-kh-pc-total",
        [
            [1000.0, 4000.0, 12000.0],
            [1000.0, 2000.0, 4000.0],
            [NAN, 1000.0, 2000.0],
            [1000.0, 2000.0, 4000.0],
        ],
    );
    let health = entry(&out, "wdi-sh-xpd-chex-pc-cd-total");
    assert_eq!(
        (health["title"].as_str(), health["unit"].as_str()),
        (Some("Health spending"), Some("current US$"))
    );
    let power = entry(&out, "wdi-eg-use-elec-kh-pc-total");
    assert_eq!(
        (power["title"].as_str(), power["unit"].as_str()),
        (Some("Electric power use"), Some("kWh"))
    );
}

#[test]
fn index_carries_the_derived_sentence_only_on_derived_series() {
    let (out, _) = run_and_write("rates-index");
    let internet = entry(&out, "wdi-it-net-user-zs-total");
    assert_eq!(internet["derived"], "share of population times population");
    assert_eq!(internet["title"], "Individuals using the Internet");
    assert_eq!(internet["unit"], "people");
    assert_eq!(internet["source"], "World Bank WDI");
    assert_eq!(
        (
            internet["first"].as_u64(),
            internet["last"].as_u64(),
            internet["entities"].as_u64()
        ),
        (Some(2019), Some(2021), Some(4))
    );
    let labor = entry(&out, "wdi-sl-uem-totl-zs-total");
    assert_eq!(labor["title"], "Unemployment, total (modeled ILO estimate)");
    assert_eq!(labor["derived"], "share of labor force times labor force");
    let native = entry(&out, "wdi-sp-pop-totl");
    assert!(native.get("derived").is_none());
    let with_derived = entries(&out)
        .iter()
        .filter(|e| e.get("derived").is_some())
        .count();
    assert_eq!(with_derived, 15);
}

#[test]
fn rates_that_are_not_totals_are_not_converted() {
    let (out, _) = run_and_write("rates-skipped");
    let slugs: Vec<String> = entries(&out)
        .iter()
        .map(|e| e["slug"].as_str().unwrap().to_string())
        .collect();
    for code in [
        "dig-idx",
        "ny-gdp-grow",
        "sh-xpd-pc-cn",
        "sh-age-std-p5",
        "per-cov-q1",
        "lic-user-zs",
    ] {
        assert!(
            !slugs.iter().any(|s| s.starts_with(&format!("wdi-{code}"))),
            "{code}"
        );
    }
}

#[test]
fn derived_series_dropped_for_negatives_coverage_and_native_twins() {
    let (out, _) = run_and_write("rates-dropped");
    let slugs: Vec<String> = entries(&out)
        .iter()
        .map(|e| e["slug"].as_str().unwrap().to_string())
        .collect();
    for dropped in [
        "wdi-gc-lend-gd-zs-total",
        "wdi-rem-sites-gd-zs-total",
        "wdi-ny-gdp-pcap-cd-total",
        "wdi-sp-urb-totl-in-zs-total",
    ] {
        assert!(!slugs.contains(&dropped.to_string()), "{dropped}");
        assert!(
            !out.join(format!("series/{dropped}.bin")).exists(),
            "{dropped}"
        );
    }
    assert!(slugs.contains(&"wdi-ny-gdp-mktp-cd".to_string()));
    assert!(slugs.contains(&"wdi-sp-urb-totl".to_string()));
}

#[test]
fn twins_are_retitled_by_tag_or_merged() {
    let (out, _) = run_and_write("rates-twins");
    let exports = entry(&out, "wdi-bx-tran-zs-total");
    let imports = entry(&out, "wdi-bm-tran-zs-total");
    assert_eq!(
        exports["title"],
        "Transport services, share of service exports"
    );
    assert_eq!(
        imports["title"],
        "Transport services, share of service imports"
    );
    assert_eq!(exports["unit"], imports["unit"]);
    assert_series(
        &out,
        "wdi-bx-tran-zs-total",
        [
            [100.0, 200.0, 400.0],
            [50.0; 3],
            [10.0, 20.0, 40.0],
            [1.0, 2.0, 4.0],
        ],
    );
    let all = entries(&out);
    let mut keys: Vec<(String, String)> = all
        .iter()
        .map(|e| (e["title"].to_string(), e["unit"].to_string()))
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), all.len());
    assert!(all.iter().any(|e| e["slug"] == "wdi-hs-a-total"));
    assert!(!all.iter().any(|e| e["slug"] == "wdi-hs-b-total"));
}

#[test]
fn flows_and_changes_and_licences() {
    let (out, _) = run_and_write("rates-flows");
    let trade = entry(&out, "wdi-ne-trd-gnfs-zs-total");
    assert_eq!(trade["title"], "Trade, exports plus imports");
    assert_series(
        &out,
        "wdi-ne-trd-gnfs-zs-total",
        [
            [100000.0, 200000.0, 400000.0],
            [50000.0; 3],
            [10000.0, 20000.0, 40000.0],
            [1000.0, 2000.0, 4000.0],
        ],
    );
    let slugs: Vec<String> = entries(&out)
        .iter()
        .map(|e| e["slug"].as_str().unwrap().to_string())
        .collect();
    for code in [
        "credit-grow-zs",
        "deaths-adj-p3",
        "beds-ch-p3",
        "mrch-rate-zs",
    ] {
        assert!(!slugs.contains(&format!("wdi-{code}-total")), "{code}");
    }
}

#[test]
fn a_slug_shared_by_native_and_derived_is_an_error() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rates");
    let dir = out_dir("rates-slug");
    fs::create_dir_all(&dir).unwrap();
    for name in ["WDICountry.csv", "WDISeries.csv", "WDICSV.csv"] {
        fs::copy(src.join(name), dir.join(name)).unwrap();
    }
    let mut series = fs::read_to_string(dir.join("WDISeries.csv")).unwrap();
    series.push_str("SH.MED.BEDS.ZS-TOTAL,Misc,Spare beds,x,Sum,CC BY-4.0\n");
    fs::write(dir.join("WDISeries.csv"), series).unwrap();
    let mut data = fs::read_to_string(dir.join("WDICSV.csv")).unwrap();
    for c in ["AAA", "BBB", "CCC", "DDD"] {
        data.push_str(&format!("x,{c},Spare beds,SH.MED.BEDS.ZS-TOTAL,1,1,1\n"));
    }
    fs::write(dir.join("WDICSV.csv"), data).unwrap();
    let err = wdi::run(&dir, 3)
        .expect_err("collision must fail")
        .to_string();
    assert!(
        err.contains("SH.MED.BEDS.ZS-TOTAL") && err.contains("SH.MED.BEDS.ZS"),
        "{err}"
    );
}

#[test]
fn aggregate_rows_never_reach_a_derived_series() {
    let (out, _) = run_and_write("rates-aggregates");
    for slug in ["wdi-it-net-user-zs-total", "wdi-sp-pop-totl"] {
        let (codes, _, _, _) = read_series(&out, slug);
        assert!(!codes.contains(&"WLD".to_string()), "{slug}");
    }
}
