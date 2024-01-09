use std::{ffi::OsStr, path::PathBuf, process::Command};

use crate::{password::get_password, WPass};
use anyhow::{anyhow, Result};
use encoding::{all::GBK, decode, DecoderTrap};
use log::{debug, info};
use rayon::prelude::*;

enum ReturnCode {
    Success = 0,
    FatalError = 2,
}

#[derive(Debug)]
pub struct WPassInstance {
    /// Password file path, use default password file if not set
    password_file: PathBuf,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    executable_path: PathBuf,
}

impl WPassInstance {
    pub fn new(password_file: PathBuf, executable_path: PathBuf) -> Self {
        Self {
            password_file,
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
    fn try_extract(&self, target: &PathBuf, output: &PathBuf) -> Result<bool> {
        let password_dict = get_password(&self.password_file).expect("Cannot read passwords");
        debug!("Password list: {:?}", password_dict);

        let password = password_dict.par_iter().find_any(|password| -> bool {
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
            self.extract(target, output, password)
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
    debug!("Stdout: {}", stdout);
    debug!("Stderr: {}", stderr);
    match output.status.code() {
        Some(0) => Ok(ReturnCode::Success),
        Some(2) => Ok(ReturnCode::FatalError),
        _ => Err(anyhow!("Unknown return code from 7zip")),
    }
}
