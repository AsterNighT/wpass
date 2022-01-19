use std::{fs, path::PathBuf};
use anyhow::Result;

pub type PasswordDict = Vec<String>;

pub fn get_password(file_path: &PathBuf) -> Result<PasswordDict>{
    let contents = fs::read_to_string(file_path)?;
    let lines = contents.split("\n");
    let vec = lines.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect::<Vec<String>>();
    Ok(vec)
}

#[test]
fn should_read_file() -> Result<()> {
    let file_path = PathBuf::from("test/dict.txt");
    let passwords = get_password(&file_path)?;
    assert_eq!(passwords, vec!["aaa","bbb"]);
    Ok(())
}
