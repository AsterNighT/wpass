mod extract;
mod password;

use anyhow::Result;
use std::path::PathBuf;
pub use extract::WPassInstance;
pub use password::get_password;

pub trait WPass {
    fn try_extract(&self, target:&PathBuf, output:&PathBuf) -> Result<bool>;
}

