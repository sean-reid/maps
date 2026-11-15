use pipeline::{fetch, index, wdi, Result};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "usage:
  pipeline wdi --input <dir> --out <dir> [--min-countries <n>]
  pipeline fetch-wdi --to <dir>";

fn flag(args: &[String], name: &str) -> Option<String> {
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1).cloned()
}

fn required(args: &[String], name: &str) -> Result<PathBuf> {
    flag(args, name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}\n{USAGE}").into())
}

fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("wdi") => {
            let input = required(args, "--input")?;
            let out = required(args, "--out")?;
            let min_countries = match flag(args, "--min-countries") {
                Some(v) => v.parse()?,
                None => wdi::DEFAULT_MIN_COUNTRIES,
            };
            let start = Instant::now();
            let (indicators, stats) = wdi::run(&input, min_countries)?;
            let bytes = index::write(&out, &indicators)?;
            println!("{stats}");
            println!(
                "wrote {} series, {bytes} bytes, in {:.1}s",
                indicators.len(),
                start.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Some("fetch-wdi") => fetch::fetch(&required(args, "--to")?),
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
