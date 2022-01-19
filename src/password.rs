use std::{fs, path::PathBuf};
use anyhow::Result;

pub type PasswordDict = Vec<String>;

pub fn get_password(file_path: &PathBuf) -> Result<PasswordDict>{
    let contents = fs::read_to_string(file_path)?;
    let lines = contents.split("\n");
    let vec = lines.map(|s| s.trim().to_owned()).collect::<Vec<String>>();
    Ok(vec)
}