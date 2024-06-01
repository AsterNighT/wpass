mod extract;
mod password;

use anyhow::Result;
use std::path::PathBuf;
use anyhow::anyhow;
pub use extract::WPassInstance;
pub use password::get_password;

#[derive(Debug)]
pub struct TestResult {
    pub password: String,
    pub size_in_bytes: usize,
    pub volumes: usize,
}

pub trait WPass: Clone {
    /// Returns the successfully extracted archives
    fn try_extract(&self, target:&PathBuf, output:&PathBuf) -> Result<(String, Vec<PathBuf>)>;
    fn extract_with_password(&self, target:&PathBuf, output:&PathBuf, password: &String) -> Result<(String, Vec<PathBuf>)>;
    /// Force extract, ignoring if it is an archive or not
    fn extract_blindly(&self, target:&PathBuf, output:&PathBuf) -> Result<(String, Vec<PathBuf>)>;
    /// Test if a file is extractable, with `7z -t`, return password if ok
    fn test(&self, target:&PathBuf) -> Result<TestResult>;
}

pub fn parse_stdout(stdout: &String) -> Result<StdoutInfo> {
    // Total Physical Size = 48407393813
    let total_size_regex = regex::Regex::new(r"Total Physical Size = (\d+)")?;
    let size_regex = regex::Regex::new(r"Physical Size = (\d+)")?;
    // Volumes = 3
    let volume_regex = regex::Regex::new(r"Volumes = (\d+)")?;
    let size = if let Some(capture) = total_size_regex.captures(stdout) {
        capture.get(1).unwrap().as_str().parse()?
    } else {
        size_regex
            .captures(stdout)
            .ok_or(anyhow!("Cannot find size in stdout"))?
            .get(1)
            .unwrap()
            .as_str()
            .parse()?
    };
    let volumes = if let Some(capture) = volume_regex.captures(stdout) {
        capture.get(1).unwrap().as_str().parse()?
    } else {
        1
    };
    Ok(StdoutInfo {
        size_in_bytes: size,
        volumes,
    })
}

pub struct StdoutInfo {
    pub size_in_bytes: usize,
    pub volumes: usize,
}
