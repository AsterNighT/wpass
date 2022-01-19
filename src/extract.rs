use std::{process::Command};

use crate::{password::get_password, CommandLineArgument};
use anyhow::{anyhow, Result};
use log::{debug, info};

enum ReturnCode {
    Success = 0,
    FatalError = 2,
    Unknown = 255,
}

pub fn try_extract(options: &CommandLineArgument) -> bool {
    let password_dict = get_password(&options.password_file).expect("Cannot read passwords");
    debug!("Password list: {:?}", password_dict);
    password_dict.into_iter().any(|password| -> bool {
        debug!("Trying password {}", &password);
        match try_password(options, &password) {
            Ok(true) => {
                info!("Password is: {}", &password);
                true
            },
            Ok(false) => false,
            Err(e) => {
                debug!("Error occurs while extracting: {}", e);
                false
            }
        }
    })
}

fn try_password(options: &CommandLineArgument, password: &str) -> Result<bool> {
    match call_7z(options, password)? {
        ReturnCode::Success => Ok(true),
        ReturnCode::FatalError => Ok(false),
        _ => Err(anyhow!("Unknown return code from 7z")),
    }
}

fn call_7z(options: &CommandLineArgument, password: &str) -> Result<ReturnCode> {
    let mut command = Command::new(&options.executable_path);
    command.arg("x");
    command.arg(format!("-p{}", password));
    command.arg("-y");
    command.arg(&options.file_path);
    command.arg(format!("-o{}",&options.output.to_str().unwrap()));
    let output = command.output()?;
    debug!("Stdout: {}", std::str::from_utf8(&output.stdout)?);
    debug!("Stderr: {}", std::str::from_utf8(&output.stderr)?);
    match output.status.code() {
        Some(0) => Ok(ReturnCode::Success),
        Some(2) => Ok(ReturnCode::FatalError),
        _ => Ok(ReturnCode::Unknown),
    }
}
