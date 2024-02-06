use std::{ffi::OsStr, os::windows::process::CommandExt, path::PathBuf, process::Command};

use crate::{password::PasswordDict, WPass};
use anyhow::{anyhow, Result};
use encoding::{all::GBK, decode, DecoderTrap};
use log::{debug, info};
use rayon::prelude::*;
const CREATE_NO_WINDOW: u32 = 0x08000000;

enum ReturnCode {
    Success = 0,
    FatalError = 2,
}

#[derive(Debug)]
pub struct WPassInstance {
    /// Possible passwords
    password_dict: PasswordDict,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    executable_path: PathBuf,
}

impl WPassInstance {
    pub fn new(password_dict: PasswordDict, executable_path: PathBuf) -> Self {
        Self {
            password_dict,
            executable_path,
        }
    }

    fn try_password(&self, target: &PathBuf, password: &str) -> Result<bool> {
        let mut command = Command::new(&self.executable_path);
        command.arg("t");
        command.arg(format!("-p{}", password));
        command.arg(target);
        Ok(parse_return_code(call_7z(&mut command)?))
    }

    fn extract(&self, target: &PathBuf, output: &PathBuf, password: &str) -> Result<bool> {
        let mut command = Command::new(&self.executable_path);
        command.arg("x");
        command.arg("-y");
        command.arg(format!("-p{}", password));
        command.arg(target);
        command.arg(format!("-o{}", output.to_str().unwrap()));
        Ok(parse_return_code(call_7z(&mut command)?))
    }
}

impl WPass for WPassInstance {
    fn try_extract(&self, target: &PathBuf, output: &PathBuf) -> Result<Vec<PathBuf>> {
        debug!("Password list: {:?}", self.password_dict);
        let archives = find_all_volumes(target);
        debug!("Archives: {:?}", archives);
        let target = archives.first().unwrap();
        let password = self.password_dict.par_iter().find_any(|password| -> bool {
            debug!("Trying password {}", password);
            match self.try_password(&target, password) {
                Ok(true) => {
                    info!("Password is: {}", password);
                    true
                }
                Ok(false) => false,
                Err(e) => {
                    debug!("Error occurs while extracting: {}", e);
                    false
                }
            }
        });
        if let Some(password) = password {
            match self.extract(target, output, password) {
                Ok(true) => {
                    info!("Successfully extracted");
                    Ok(archives)
                }
                Ok(false) => Err(anyhow!("Cannot extract with correct password")),
                Err(e) => Err(e),
            }
        } else {
            Err(anyhow!("Cannot find correct password"))
        }
    }
}

fn parse_return_code(code: ReturnCode) -> bool {
    match code {
        ReturnCode::Success => true,
        _ => false,
    }
}

fn call_7z(command: &mut Command) -> Result<ReturnCode> {
    #[cfg(not(debug_assertions))]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.output()?;
    debug!("args: {:?}", command.get_args().collect::<Vec<&OsStr>>());

    // This is clearly not the best way to do this, I wonder if there's a way to change code page to 65001 on windows
    // Leave it as a TODO
    let stdout = decode(&output.stdout, DecoderTrap::Strict, GBK).0.unwrap();
    let stderr = decode(&output.stderr, DecoderTrap::Strict, GBK).0.unwrap();
    // let mut stdout = String::new();
    // let mut stderr = String::new();
    // GBK.decode_to(&output.stdout, DecoderTrap::Replace, &mut stdout)
    //     .unwrap();
    // GBK.decode_to(&output.stderr, DecoderTrap::Replace, &mut stderr)
    //     .unwrap();
    if output.status.code() != Some(0) {
        log::error!("Stdout: {}", stdout);
        log::error!("Stderr: {}", stderr);
    } else {
        log::debug!("Stdout: {}", stdout);
        log::debug!("Stderr: {}", stderr);
    }
    match output.status.code() {
        Some(0) => Ok(ReturnCode::Success),
        Some(2) => Ok(ReturnCode::FatalError),
        _ => Err(anyhow!("Unknown return code from 7zip")),
    }
}

/// Surprisingly, after a thorough search, I cannot find a library to do one simple thing:
/// Finding all relevant volumes of one multipart archive.
/// I guess I have to do it myself.
/// Put it here because this seems quite useful for any interface.
/// The returned vector should be sorted in order. Which means the first element is the first volume.
/// And it is the first volume should be fed to 7z
/// If there is nothing we could find just return what is provided.
pub fn find_all_volumes(volume: &PathBuf) -> Vec<PathBuf> {
    let dir = volume.parent().unwrap();
    let base_name = volume.file_stem().unwrap();
    let files = std::fs::read_dir(dir).unwrap();
    let mut result:Vec<_> = files.filter_map(|entry| {
        if let Ok(entry) = entry {
            // entry.metadata()
            let path = entry.path();
            if path.is_file() && path.file_stem().unwrap() == base_name {
                return Some(path);
            }
        }
        None
    }).collect();
    result.sort();
    result
}
