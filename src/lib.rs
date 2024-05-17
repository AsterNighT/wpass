mod extract;
mod password;

use anyhow::Result;
use std::path::PathBuf;
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
    fn try_extract(&self, target:&PathBuf, output:&PathBuf) -> Result<Vec<PathBuf>>;
    fn extract_with_password(&self, target:&PathBuf, output:&PathBuf, password: &String) -> Result<Vec<PathBuf>>;
    /// Test if a file is extractable, with `7z -t`, return password if ok
    fn test(&self, target:&PathBuf) -> Result<TestResult>;
}

