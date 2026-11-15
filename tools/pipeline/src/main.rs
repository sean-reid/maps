use pipeline::build::{self, Inputs};
use pipeline::{fetch, index, wdi, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "usage:
  pipeline build --wdi <dir> --out <dir> [--min-countries <n>]
  pipeline fetch-wdi --to <dir>";

/// Parses `--flag value` pairs; an unknown flag, a missing value, or a value that looks like a flag is an error.
fn parse_flags(args: &[String], known: &[&str]) -> Result<HashMap<String, String>> {
    let mut flags = HashMap::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if !known.contains(&arg.as_str()) {
            return Err(format!("unexpected argument {arg}\n{USAGE}").into());
        }
        match it.next() {
            Some(v) if !v.starts_with("--") => flags.insert(arg.clone(), v.clone()),
            _ => return Err(format!("{arg} needs a value\n{USAGE}").into()),
        };
    }
    Ok(flags)
}

fn required(flags: &HashMap<String, String>, name: &str) -> Result<PathBuf> {
    flags
        .get(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}\n{USAGE}").into())
}

fn run(args: &[String]) -> Result<()> {
    let rest = args.get(1..).unwrap_or(&[]);
    match args.first().map(String::as_str) {
        Some("build") => {
            let flags = parse_flags(rest, &["--wdi", "--out", "--min-countries"])?;
            let wdi_dir = required(&flags, "--wdi")?;
            let out = required(&flags, "--out")?;
            let min_countries = match flags.get("--min-countries") {
                Some(v) => v.parse()?,
                None => wdi::DEFAULT_MIN_COUNTRIES,
            };
            let start = Instant::now();
            let inputs = Inputs { wdi: &wdi_dir };
            let (indicators, reports) = build::run(&inputs, min_countries)?;
            let bytes = index::write(&out, &indicators)?;
            for report in reports {
                println!("{report}");
            }
            println!(
                "wrote {} series, {bytes} bytes, in {:.1}s",
                indicators.len(),
                start.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Some("fetch-wdi") => {
            let flags = parse_flags(rest, &["--to"])?;
            fetch::fetch(&required(&flags, "--to")?)
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_pairs() {
        let f = parse_flags(
            &args(&["--input", "a", "--out", "b"]),
            &["--input", "--out"],
        )
        .unwrap();
        assert_eq!(f["--input"], "a");
        assert_eq!(f["--out"], "b");
    }

    #[test]
    fn rejects_missing_value() {
        assert!(parse_flags(&args(&["--input"]), &["--input"]).is_err());
    }

    #[test]
    fn rejects_flag_as_value() {
        assert!(parse_flags(&args(&["--input", "--out", "b"]), &["--input", "--out"]).is_err());
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(parse_flags(&args(&["--bogus", "x"]), &["--input"]).is_err());
        assert!(parse_flags(&args(&["stray"]), &["--input"]).is_err());
    }

    #[test]
    fn run_fails_on_bad_flags() {
        assert!(run(&args(&["build", "--wdi"])).is_err());
        assert!(run(&args(&["build", "--out", "x"])).is_err());
        assert!(run(&args(&["wdi", "--input", "x"])).is_err());
        assert!(run(&args(&["fetch-wdi", "--nope", "x"])).is_err());
    }
}
