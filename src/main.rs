use std::{io::Write, path::PathBuf, process::exit};

use anyhow::Result;
use clap::Parser;
use config::Config;
use log::{debug, LevelFilter};
use serde::Deserialize;
use wpass::{
    hack::{format_password_file, generate_reg},
    WPass, WPassInstance,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdArgument {
    /// Archive file path
    #[clap(parse(from_os_str))]
    file_path: PathBuf,

    /// Password file path, use default password file if not set
    #[clap(short, long, parse(from_os_str))]
    password_file: Option<PathBuf>,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    #[clap(short, long, parse(from_os_str))]
    executable_path: Option<PathBuf>,

    /// Extraction destination, use current directory if not set
    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

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
    #[clap(short = 'D', long)]
    delete: bool,

    /// Generate reg file for windows context menu. With this option enabled the program will not try to extract file, but you still need to provide an arbitrary file name.
    #[clap(short, long)]
    generate: bool,

    /// Format the password file after everything. Sort passwords and deduplicate them. Enabled by default.
    #[clap(short, long)]
    format: bool,
}

#[derive(Debug)]
pub struct CmdArgumentMerged {
    /// Archive file path
    file_path: PathBuf,

    /// Password file path, use default password file if not set
    password_file: PathBuf,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    executable_path: PathBuf,

    /// Extraction destination, use current directory if not set
    output: PathBuf,

    /// Extract to the same directory of archive file, overwrites the -o option
    local: bool,

    /// Always extract to a new directory, with same name as the archive file
    new_directory: bool,

    /// Turn debugging information on
    debug: usize,

    /// Delete the original archive file after extraction succeeds.
    delete: bool,

    /// Generate reg file for windows context menu. With this option enabled the program will not try to extract file, but you still need to provide an arbitrary file name.
    generate: bool,

    /// Format the password file after everything. Sort passwords and deduplicate them. Enabled by default.
    format: bool,
}

#[derive(Debug, Deserialize)]
pub struct WPassDefaultConfig {
    executable_path: PathBuf,
    password_file: PathBuf,
}

fn main() -> Result<()> {
    let mut config = Config::default();
    let mut config_path = std::env::current_exe().unwrap();
    config_path.pop();
    config_path.push("config.toml");
    config
        .merge(config::File::with_name(config_path.to_str().unwrap()))
        .unwrap();
    let config = config.try_into::<WPassDefaultConfig>().unwrap();
    let args = CmdArgument::parse();
    if args.debug > 0 {
        env_logger::Builder::new()
            .filter_level(LevelFilter::Debug)
            .init();
    } else {
        env_logger::init();
    }
    let (merged_args, wpass_instance) = initialize(args, config)?;
    // After initialize args should not contain any None
    let success = wpass(&wpass_instance, &merged_args)?;
    if success {
        finalize(&merged_args).expect("Fail to finalize");
    }
    // System("pause")
    if merged_args.debug >= 1 || !success {
        let mut stdout = std::io::stdout();
        stdout.write(b"Press Enter to continue...").unwrap();
        stdout.flush().unwrap();
        std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
    }
    Ok(())
}

pub fn initialize(
    options: CmdArgument,
    config: WPassDefaultConfig,
) -> Result<(CmdArgumentMerged, WPassInstance)> {
    // These unwraps really sucks.
    debug!("Read config: {:?}", config);
    debug!("Before initialization: {:?}", options);
    let mut args_merged: CmdArgumentMerged = CmdArgumentMerged {
        file_path: options.file_path.clone(),
        password_file: options.password_file.unwrap_or({
            let config_path = PathBuf::from(&config.password_file);
            if config_path.is_absolute() {
                config_path
            } else {
                let mut path = std::env::current_exe().expect("Cannot get exe path");
                path.pop();
                path.push(config_path);
                path
            }
        }),
        executable_path: options.executable_path.unwrap_or({
            let config_path = PathBuf::from(&config.executable_path);
            if config_path.is_absolute() {
                config_path
            } else {
                let mut path = std::env::current_exe().expect("Cannot get exe path");
                path.pop();
                path.push(config_path);
                path
            }
        }),
        output: options.output.unwrap_or(std::env::current_dir()?),
        local: options.local,
        new_directory: options.new_directory,
        debug: options.debug,
        delete: options.delete,
        generate: options.generate,
        format: options.format,
    };

    if args_merged.local {
        args_merged.output = {
            let mut file_dir = args_merged.file_path.clone();
            file_dir.pop();
            file_dir
        };
        // This may make the output empty string. 7z will complain about it. So make it a directory.
        if !args_merged.output.is_dir() {
            args_merged.output.push(".");
            assert!(args_merged.output.is_dir());
        }
    }

    if args_merged.new_directory {
        match args_merged.file_path.file_stem() {
            Some(filename) => {
                args_merged.output.push(filename);
                // Some times the file has no extension, in which case the directory will have the same name as the file itself, we will fail to create the directory.
                if std::path::Path::exists(&args_merged.output) {
                    args_merged.output.pop();
                    args_merged
                        .output
                        .push(format!("{}_extracted", filename.to_str().unwrap()));
                }
            }
            // What the hell?
            None => args_merged.output.push("foobar"),
        }
    }
    debug!("After initialization: {:?}", args_merged);
    let wpass = WPassInstance::new(
        args_merged.password_file.clone(),
        args_merged.executable_path.clone(),
    );
    Ok((args_merged, wpass))
}

fn wpass(wpass: &WPassInstance, args: &CmdArgumentMerged) -> Result<bool> {
    if args.generate {
        if cfg!(windows) {
            generate_reg("wpass.reg");
            exit(0);
        } else {
            println!("Register is only available on Windows");
            exit(1);
        }
    }
    wpass.try_extract(&args.file_path, &args.output)
}

fn finalize(args: &CmdArgumentMerged) -> Result<()> {
    if args.delete {
        std::fs::remove_file(&args.file_path)?;
    }
    if args.format {
        format_password_file(&args.password_file)?;
    }
    Ok(())
}
