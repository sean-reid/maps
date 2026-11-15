use crate::fetch::{MAX_DOWNLOAD, MAX_ENTRY, TIMEOUT};
use crate::Result;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

const BASE: &str = "https://bulks-faostat.fao.org/production/";
/// (domain code, zip name); each zip holds the normalized "All Data" csv.
pub const FILES: [(&str, &str); 6] = [
    ("QCL", "Production_Crops_Livestock_E_All_Data_(Normalized)"),
    ("RL", "Inputs_LandUse_E_All_Data_(Normalized)"),
    ("FO", "Forestry_E_All_Data_(Normalized)"),
    ("RFN", "Inputs_FertilizersNutrient_E_All_Data_(Normalized)"),
    ("RP", "Inputs_Pesticides_Use_E_All_Data_(Normalized)"),
    ("GT", "Emissions_Totals_E_All_Data_(Normalized)"),
];
const PAUSE: Duration = Duration::from_secs(3);
const ATTEMPTS: u32 = 4;
const MAX_WAIT: u64 = 120;

pub fn fetch(to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent("maps-pipeline")
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    for (at, (code, name)) in FILES.iter().enumerate() {
        if at > 0 {
            sleep(PAUSE);
        }
        let zip_path = to.join(format!("{code}.zip"));
        let result = download(&agent, &format!("{BASE}{name}.zip"), &zip_path).and_then(|()| {
            unzip(
                &zip_path,
                &format!("{name}.csv"),
                &to.join(format!("{code}.csv")),
            )
        });
        let _ = fs::remove_file(&zip_path);
        result.map_err(|e| format!("{code}: {e}"))?;
    }
    Ok(())
}

fn download(agent: &ureq::Agent, url: &str, zip_path: &Path) -> Result<()> {
    for attempt in 1..=ATTEMPTS {
        let mut response = agent.get(url).call()?;
        let status = response.status().as_u16();
        if status == 200 {
            let mut reader = response
                .body_mut()
                .with_config()
                .limit(MAX_DOWNLOAD)
                .reader();
            io::copy(&mut reader, &mut File::create(zip_path)?)?;
            return Ok(());
        }
        let retry = status == 429 || status == 503;
        if !retry || attempt == ATTEMPTS {
            return Err(format!("{url}: status {status}").into());
        }
        let header = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok());
        sleep(Duration::from_secs(retry_wait(header, attempt)));
    }
    unreachable!()
}

/// Seconds to wait before the next attempt: the server's Retry-After when it is a number, capped.
fn retry_wait(header: Option<&str>, attempt: u32) -> u64 {
    let asked = header.and_then(|v| v.trim().parse::<u64>().ok());
    asked.unwrap_or(10 * u64::from(attempt)).min(MAX_WAIT)
}

pub fn unzip(zip_path: &Path, entry: &str, to: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    let file = archive.by_name(entry)?;
    let mut limited = file.take(MAX_ENTRY + 1);
    let written = io::copy(&mut limited, &mut File::create(to)?)?;
    if written > MAX_ENTRY {
        return Err(format!("{entry} is larger than {MAX_ENTRY} bytes").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pipeline-fao-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut w = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn extracts_the_named_entry_only() {
        let dir = scratch("unzip");
        let zip = dir.join("in.zip");
        make_zip(&zip, &[("a.csv", b"x,y\n1,2\n"), ("flags.csv", b"other")]);
        unzip(&zip, "a.csv", &dir.join("A.csv")).unwrap();
        assert_eq!(fs::read(dir.join("A.csv")).unwrap(), b"x,y\n1,2\n");
        assert!(unzip(&zip, "missing.csv", &dir.join("B.csv")).is_err());
    }

    #[test]
    fn retry_after_is_honoured_and_capped() {
        assert_eq!(retry_wait(Some("7"), 1), 7);
        assert_eq!(retry_wait(Some(" 30 "), 3), 30);
        assert_eq!(retry_wait(Some("100000"), 1), MAX_WAIT);
        assert_eq!(retry_wait(None, 2), 20);
        assert_eq!(retry_wait(Some("Wed, 21 Oct 2026 07:28:00 GMT"), 1), 10);
    }

    #[test]
    fn every_domain_file_is_a_distinct_zip() {
        let mut names: Vec<&str> = FILES.iter().map(|(_, n)| *n).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), FILES.len());
    }
}
