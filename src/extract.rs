use std::{ffi::OsStr, process::Command};

use crate::{CommandLineArgument, password::get_password};
use anyhow::{anyhow, Result};
use log::{debug, info};
use rayon::prelude::*;
use encoding::{DecoderTrap, Encoding, all::GBK};

enum ReturnCode {
    Success = 0,
    FatalError = 2,
}

pub fn try_extract(options: &CommandLineArgument) -> Result<bool> {
    let password_dict = get_password(&options.password_file).expect("Cannot read passwords");
    debug!("Password list: {:?}", password_dict);
    
    let correct_password = std::sync::Mutex::new("".to_owned());
    password_dict.par_iter().any(|password| -> bool {
        debug!("Trying password {}", password);
        match try_password(options, password) {
            Ok(true) => {
                info!("Password is: {}", password);
                *correct_password.lock().unwrap() = password.clone();
                true
            },
            Ok(false) => false,
            Err(e) => {
                debug!("Error occurs while extracting: {}", e);
                false
            }
        }
    });
    extract(options, &correct_password.into_inner().unwrap())
}

fn try_password(options: &CommandLineArgument, password: &str) -> Result<bool> {
    let mut command = Command::new(&options.executable_path);
    command.arg("t");
    command.arg(format!("-p{}", password));
    command.arg(&options.file_path);
    Ok(parse_return_code(call_7z(&mut command)?))
}

fn extract(options: &CommandLineArgument, password: &str) -> Result<bool> {
    let mut command = Command::new(&options.executable_path);
    command.arg("x");
    command.arg("-y");
    command.arg(format!("-p{}", password));
    command.arg(&options.file_path);
    command.arg(format!("-o{}",&options.output.to_str().unwrap()));
    Ok(parse_return_code(call_7z(&mut command)?))
}

fn parse_return_code(code:ReturnCode) -> bool {
    match code {
        ReturnCode::Success => true,
        _ => false,
    }
}

fn call_7z(command:&mut Command) -> Result<ReturnCode> {
    let output = command.output()?;
    debug!("args: {:?}",command.get_args().collect::<Vec<&OsStr>>());
    let mut stdout = String::new();
    let mut stderr = String::new();
    GBK.decode_to(&output.stdout,DecoderTrap::Replace, &mut stdout).unwrap();
    GBK.decode_to(&output.stderr,DecoderTrap::Replace, &mut stderr).unwrap();
    debug!("Stdout: {}", stdout);
    debug!("Stderr: {}", stderr);
    match output.status.code() {
        Some(0) => Ok(ReturnCode::Success),
        Some(2) => Ok(ReturnCode::FatalError),
        _ => Err(anyhow!("Unknown return code from 7zip")),
    }
}