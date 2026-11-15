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
                MAX_ENTRY,
            )
        });
        let _ = fs::remove_file(&zip_path);
        result.map_err(|e| format!("{code}: {e}"))?;
    }
    Ok(())
}

/// Why an attempt failed.
#[derive(Debug, PartialEq)]
enum Failure {
    Status(u16),
    /// A connection error, a timeout, or a body that broke off.
    Transport,
    /// The body passed the size cap or the file cannot be written; asking again would repeat it.
    Final,
}

/// Seconds to wait before another attempt, or None when the failure is final or attempts ran out.
fn next_wait(failure: &Failure, retry_after: Option<&str>, attempt: u32) -> Option<u64> {
    let retryable = match failure {
        Failure::Status(code) => [429, 500, 502, 503, 504].contains(code),
        Failure::Transport => true,
        Failure::Final => false,
    };
    (retryable && attempt < ATTEMPTS).then(|| retry_wait(retry_after, attempt))
}

fn copy_body(
    response: &mut ureq::http::Response<ureq::Body>,
    to: &Path,
) -> std::result::Result<(), (Failure, String)> {
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .reader();
    let mut file = File::create(to).map_err(|e| (Failure::Final, e.to_string()))?;
    io::copy(&mut reader, &mut file).map(|_| ()).map_err(|e| {
        let capped = e
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<ureq::Error>())
            .is_some_and(|inner| matches!(inner, ureq::Error::BodyExceedsLimit(_)));
        let failure = if capped {
            Failure::Final
        } else {
            Failure::Transport
        };
        (failure, e.to_string())
    })
}

fn download(agent: &ureq::Agent, url: &str, zip_path: &Path) -> Result<()> {
    let mut attempt = 1;
    loop {
        let mut retry_after = None;
        let (failure, detail) = match agent.get(url).call() {
            Err(e) => (Failure::Transport, e.to_string()),
            Ok(mut response) => {
                let status = response.status().as_u16();
                retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                if status == 200 {
                    match copy_body(&mut response, zip_path) {
                        Ok(()) => return Ok(()),
                        Err(failed) => failed,
                    }
                } else {
                    (Failure::Status(status), format!("status {status}"))
                }
            }
        };
        match next_wait(&failure, retry_after.as_deref(), attempt) {
            Some(wait) => sleep(Duration::from_secs(wait)),
            None => return Err(format!("{url}: {detail}").into()),
        }
        attempt += 1;
    }
}

/// Seconds to wait before the next attempt: the server's Retry-After when it is a number, capped.
fn retry_wait(header: Option<&str>, attempt: u32) -> u64 {
    let asked = header.and_then(|v| v.trim().parse::<u64>().ok());
    asked.unwrap_or(10 * u64::from(attempt)).min(MAX_WAIT)
}

/// Extracts `entry` to `to`; a failed or oversized extraction leaves no file behind.
pub fn unzip(zip_path: &Path, entry: &str, to: &Path, max_entry: u64) -> Result<()> {
    let result = extract(zip_path, entry, to, max_entry);
    if result.is_err() {
        let _ = fs::remove_file(to);
    }
    result
}

fn extract(zip_path: &Path, entry: &str, to: &Path, max_entry: u64) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    let file = archive.by_name(entry)?;
    let mut limited = file.take(max_entry + 1);
    let written = io::copy(&mut limited, &mut File::create(to)?)?;
    if written > max_entry {
        return Err(format!("{entry} is larger than {max_entry} bytes").into());
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
        unzip(&zip, "a.csv", &dir.join("A.csv"), 1024).unwrap();
        assert_eq!(fs::read(dir.join("A.csv")).unwrap(), b"x,y\n1,2\n");
        assert!(unzip(&zip, "missing.csv", &dir.join("B.csv"), 1024).is_err());
        assert!(!dir.join("B.csv").exists());
    }

    #[test]
    fn an_oversized_entry_leaves_no_partial_file() {
        let dir = scratch("big");
        let zip = dir.join("in.zip");
        make_zip(&zip, &[("a.csv", b"0123456789")]);
        let err = unzip(&zip, "a.csv", &dir.join("A.csv"), 4).unwrap_err();
        assert!(err.to_string().contains("a.csv"), "{err}");
        assert!(!dir.join("A.csv").exists());
    }

    #[test]
    fn server_errors_and_transport_failures_retry_within_the_attempt_bound() {
        for code in [429, 500, 502, 503, 504] {
            assert_eq!(
                next_wait(&Failure::Status(code), Some("7"), 1),
                Some(7),
                "{code}"
            );
        }
        assert_eq!(next_wait(&Failure::Transport, None, 2), Some(20));
        assert_eq!(
            next_wait(&Failure::Status(503), Some("999"), 3),
            Some(MAX_WAIT)
        );
        for failure in [Failure::Status(503), Failure::Transport] {
            assert_eq!(next_wait(&failure, None, ATTEMPTS), None);
        }
    }

    #[test]
    fn client_errors_and_final_failures_do_not_retry() {
        for code in [400, 403, 404, 410, 501] {
            assert_eq!(next_wait(&Failure::Status(code), None, 1), None, "{code}");
        }
        assert_eq!(next_wait(&Failure::Final, None, 1), None);
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
