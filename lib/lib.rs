mod extract;
pub mod hack;
mod password;

use anyhow::Result;
use std::path::PathBuf;
pub use extract::WPassInstance;

pub trait WPass {
    fn try_extract(&self, target:&PathBuf, output:&PathBuf) -> Result<bool>;
}

extern "C" {
    pub fn system(command: *const u8);
}

