use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn destination_from_file_path(
    file_path: &PathBuf,
    output_path: &Option<PathBuf>,
    local: bool,
    new_directory: bool,
) -> Result<PathBuf> {
    assert!(file_path.is_file());
    let mut destination = output_path.clone().unwrap_or(std::env::current_dir()?);
    if local {
        destination = file_path.clone();
        destination.pop();
        // This may make the output empty string. 7z will complain about it. So make it a directory.
        if !destination.is_dir() {
            destination.push(".");
            assert!(destination.is_dir());
        }
    }

    if new_directory {
        match file_path.file_stem() {
            Some(filename) => {
                destination.push(filename);
                // Some times the file has no extension, in which case the directory will have the same name as the file itself, we will fail to create the directory.
                if std::path::Path::exists(&destination) {
                    destination.pop();
                    destination.push(format!("{}_extracted", filename.to_str().unwrap()));
                }
            }
            // What the hell?
            None => destination.push("foobar"),
        }
    };
    Ok(destination)
}

pub fn get_all_files_from_directory(directory: &PathBuf) -> Result<Vec<PathBuf>> {
    // This handle the case where the directory is a file.
    Ok(WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|e| e.is_file())
        .collect())
}
