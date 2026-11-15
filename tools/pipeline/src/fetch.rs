use crate::Result;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

pub const URL: &str = "https://databank.worldbank.org/data/download/WDI_CSV.zip";
const WANTED: [&str; 3] = ["WDICSV.csv", "WDISeries.csv", "WDICountry.csv"];
const MAX_DOWNLOAD: u64 = 1 << 30;
const MAX_ENTRY: u64 = 2 << 30;
const TIMEOUT: Duration = Duration::from_secs(600);

pub fn fetch(to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    let zip_path = to.join("WDI_CSV.zip");
    let result = download(&zip_path).and_then(|()| unzip(&zip_path, to, MAX_ENTRY));
    let _ = fs::remove_file(&zip_path);
    result
}

fn download(zip_path: &Path) -> Result<()> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent("maps-pipeline")
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut body = agent.get(URL).call()?.into_body();
    let mut reader = body.with_config().limit(MAX_DOWNLOAD).reader();
    io::copy(&mut reader, &mut File::create(zip_path)?)?;
    Ok(())
}

pub fn unzip(zip_path: &Path, to: &Path, max_entry: u64) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    for name in WANTED {
        let entry = archive.by_name(name)?;
        let mut limited = entry.take(max_entry + 1);
        let written = io::copy(&mut limited, &mut File::create(to.join(name))?)?;
        if written > max_entry {
            return Err(format!("{name} is larger than {max_entry} bytes").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(dir: &Path, files: &[(&str, &[u8])]) -> std::path::PathBuf {
        let path = dir.join("in.zip");
        let mut w = zip::ZipWriter::new(File::create(&path).unwrap());
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
        path
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pipeline-fetch-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn all_files<'a>() -> Vec<(&'a str, &'a [u8])> {
        vec![
            ("WDICSV.csv", b"a,b\n1,2\n"),
            ("WDISeries.csv", b"s"),
            ("WDICountry.csv", b"c"),
            ("WDIfootnote.csv", b"ignored"),
        ]
    }

    #[test]
    fn extracts_wanted_files_only() {
        let dir = scratch("ok");
        let zip = make_zip(&dir, &all_files());
        let to = dir.join("out");
        fs::create_dir_all(&to).unwrap();
        unzip(&zip, &to, 1024).unwrap();
        assert_eq!(fs::read(to.join("WDICSV.csv")).unwrap(), b"a,b\n1,2\n");
        assert_eq!(fs::read(to.join("WDICountry.csv")).unwrap(), b"c");
        assert!(!to.join("WDIfootnote.csv").exists());
    }

    #[test]
    fn refuses_oversized_entry() {
        let dir = scratch("big");
        let zip = make_zip(&dir, &all_files());
        let to = dir.join("out");
        fs::create_dir_all(&to).unwrap();
        let err = unzip(&zip, &to, 4).unwrap_err().to_string();
        assert!(err.contains("WDICSV.csv"), "{err}");
    }

    #[test]
    fn missing_entry_is_an_error() {
        let dir = scratch("missing");
        let zip = make_zip(&dir, &[("WDICSV.csv", b"x")]);
        assert!(unzip(&zip, &dir, 1024).is_err());
    }
}
