use std::{path::{Path, PathBuf}, io::Write, fs};
use anyhow::Result;

pub fn generate_reg<P:AsRef<Path>>(file_to_write: P) -> () {
    let reg_file_content = format!(
        r#"Windows Registry Editor Version 5.00

[HKEY_CLASSES_ROOT\*\shell\WPass]
"MUIVerb"="Extract with wpass"
"SubCommands"=""
"OnlyInBrowserWindow"=""

[HOKEY_CLASSES_ROOT\*\shell\WPass\shell]

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item0]
"MUIVerb"="Extract to current directory"

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item0\command]
@="{path} -l \"%1\""

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item1]
"MUIVerb"="Extract to new directory"

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item1\command]
@="{path} -l -n \"%1\""

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item2]
"MUIVerb"="Extract to current directory(with debug output)"

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item2\command]
@="{path} -n -d -l \"%1\""

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item3]
"MUIVerb"="Extract to current directory and delete the archive file"

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item3\command]
@="{path} -l -D \"%1\""

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item4]
"MUIVerb"="Extract to new directory and delete the archive file"

[HKEY_CLASSES_ROOT\*\shell\WPass\shell\Item4\command]
@="{path} -l -D -n \"%1\"""#,
        path = std::env::current_exe().unwrap().to_str().unwrap()
    );
    let mut reg_file = std::fs::File::create(file_to_write).expect("Failed to create reg file");
    reg_file
        .write(reg_file_content.as_bytes())
        .expect("Failed to write reg file");
}

/// It seems to be a very bad idea to put this function at this level
/// The reasons are:
/// 1. WPass should be a simple tool. It does one thing merely: accept parameters, and extract files.
/// 2. In practice there could be multiple instance of WPass. It doesn't make sense to call this multiple times.
pub fn format_password_file(file_path: &PathBuf) -> Result<()>{
    let contents = fs::read_to_string(file_path)?;
    let lines = contents.split("\n");
    let mut vec = lines.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect::<Vec<String>>();
    vec.sort();
    vec.dedup();
    fs::write(file_path, vec.join("\n"))?;
    Ok(())
}