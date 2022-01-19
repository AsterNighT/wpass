mod extract;
mod password;

use clap::Parser;
use extract::try_extract;
use log::{debug, LevelFilter};
use std::{path::PathBuf, process::exit};
use anyhow::Result;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CommandLineArgument {
    /// Archive file path
    #[clap(parse(from_os_str))]
    file_path: PathBuf,

    /// Password file path, use default password file if not set
    #[clap(short, long, parse(from_os_str), default_value = "none")]
    password_file: PathBuf,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    #[clap(short, long, parse(from_os_str), default_value = "none")]
    executable_path: PathBuf,

    /// Extraction destination, use current directory if not set
    #[clap(short, long, parse(from_os_str), default_value = "none")]
    output: PathBuf,

    /// Extract to the same directory of archive file, overwrites the -o option
    #[clap(short, long)]
    local: bool,

    /// Always extract to a new directory, with same name as the archive file
    #[clap(short, long)]
    new_directory: bool,

    /// Turn debugging information on
    #[clap(short, long, parse(from_occurrences))]
    debug: usize,
    
    /// Delete the original archive file after extraction succeeds.
    #[clap(short='D', long)]
    delete: bool,
}

fn main() {
    let mut args: CommandLineArgument = CommandLineArgument::parse();
    if args.debug>= 1 {
        env_logger::Builder::new().filter_level(LevelFilter::Debug).init();
    } else {
        env_logger::init();
    }
    initialize(&mut args).expect("Fail to initialize arguments");
    let success = try_extract(&args).expect("Failed to extract");
    if success {
        finalize(&args).expect("Fail to finalize");
    }
    exit(if success {0} else {1});
}

fn finalize(args: &CommandLineArgument) -> Result<()> {
    if args.delete {
        std::fs::remove_file(&args.file_path)?;
    }
    Ok(())
}

pub fn initialize(options: &mut CommandLineArgument) -> Result<()> {
    debug!("Before initialization: {:?}", options);
    if options.password_file.to_str() == Some("none") {
        options.password_file = {
            let mut path = std::env::current_exe().expect("Cannot get exe path");
            path.pop();
            path.push("dict.txt");
            path
        }
    }

    if options.executable_path.to_str() == Some("none") {
        options.executable_path = {
            let mut path = std::env::current_exe().expect("Cannot get exe path");
            path.pop();
            path.push("7za.exe");
            path
        }
    }

    if options.output.to_str() == Some("none") {
        options.output = std::env::current_dir()?;
    }

    if options.local {
        options.output = {
            let mut file_dir = options.file_path.clone();
            file_dir.pop();
            file_dir
        };
        // This may make the output empty string. 7z will complain about it. So make it a directory.
        if !options.output.is_dir() { 
            options.output.push(".");
        }
    }

    if options.new_directory {
        match options.file_path.file_stem() {
            Some(filename) =>  options.output.push(filename),
            None => options.output.push("foobar")
        }
    }
    debug!("After initialization: {:?}", options);
    Ok(())
}
