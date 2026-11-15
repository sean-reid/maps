use crate::Result;
use std::fs::{self, File};
use std::io;
use std::path::Path;

pub const URL: &str = "https://databank.worldbank.org/data/download/WDI_CSV.zip";
const WANTED: [&str; 3] = ["WDICSV.csv", "WDISeries.csv", "WDICountry.csv"];

pub fn fetch(to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    let zip_path = to.join("WDI_CSV.zip");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent("maps-pipeline")
        .build()
        .into();
    let mut body = agent.get(URL).call()?.into_body();
    io::copy(&mut body.as_reader(), &mut File::create(&zip_path)?)?;
    unzip(&zip_path, to)?;
    fs::remove_file(&zip_path)?;
    Ok(())
}

pub fn unzip(zip_path: &Path, to: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    for name in WANTED {
        let mut entry = archive.by_name(name)?;
        io::copy(&mut entry, &mut File::create(to.join(name))?)?;
    }
    Ok(())
}
