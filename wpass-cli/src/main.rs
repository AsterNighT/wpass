mod fs;
mod hack;
mod runner;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use config::Config;
use log::{debug, LevelFilter};
use serde::Deserialize;
use wpass::{get_password, WPassInstance};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CmdArgument {
    /// Archive file path
    #[clap()]
    file_path: PathBuf,

    /// Password file path, use default password file if not set
    #[clap(short, long)]
    password_file: Option<PathBuf>,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    #[clap(short, long)]
    executable_path: Option<PathBuf>,

    /// Extraction destination, use current directory if not set
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// Extract to the same directory of archive file, overwrites the -o option
    #[clap(short, long)]
    local: bool,

    /// Always extract to a new directory, with same name as the archive file
    #[clap(short, long)]
    new_directory: bool,

    /// Recursively extract all files in the directory. Ignore directory if not enabled
    #[clap(short, long)]
    recursive: bool,

    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Delete the original archive file after extraction succeeds.
    #[clap(short = 'D', long)]
    delete: bool,

    /// Generate reg file for windows context menu. With this option enabled the program will not try to extract file.
    #[clap(short, long)]
    generate: bool,

    /// Format the password file after everything. Sort passwords and deduplicate them. Enabled by default.
    #[clap(short, long)]
    format: bool,

    /// Max parallel jobs
    #[clap(short, long, default_value = "0")]
    jobs: usize,
}

#[derive(Debug, Clone)]
pub struct CmdArgumentMerged {
    /// Archive file path
    file_path: PathBuf,

    /// Password file path, use default password file if not set
    password_file: PathBuf,

    /// Path to 7z.exe or 7za.exe, use default 7za.exe if not set
    executable_path: PathBuf,

    /// Extraction destination, use current directory if not set
    output: Option<PathBuf>,

    /// Extract to the same directory of archive file, overwrites the -o option
    local: bool,

    /// Max parallel jobs
    max_thread: usize,

    /// Always extract to a new directory, with same name as the archive file
    new_directory: bool,

    /// Turn debugging information on
    debug: u8,

    /// Delete the original archive file after extraction succeeds.
    delete: bool,

    /// Format the password file after everything. Sort passwords and deduplicate them. Enabled by default.
    format: bool,
}

#[derive(Debug, Deserialize)]
pub struct WPassDefaultConfig {
    executable_path: PathBuf,
    password_file: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut config_path = std::env::current_exe().unwrap();
    config_path.pop();
    config_path.push("config.toml");
    let config = Config::builder()
        .add_source(config::File::with_name(config_path.to_str().unwrap()))
        .build()
        .unwrap();
    let config: WPassDefaultConfig = config.try_deserialize().unwrap();
    let args = CmdArgument::parse();
    if args.debug > 0 {
        env_logger::Builder::new()
            .filter_level(LevelFilter::Debug)
            .init();
    } else {
        env_logger::init();
    }
    check_args(&args).unwrap();
    if args.generate {
        if cfg!(windows) {
            hack::generate_reg(args.file_path);
            return Ok(());
        } else {
            return Err(anyhow!("Register is only available on Windows"));
        }
    }

    let (merged_args, wpass_instance) = initialize(args, config).unwrap();
    let files = fs::get_all_files_from_directory(&merged_args.file_path).unwrap();

    debug!("Files: {:?}", files);

    let mut runner = runner::Scheduler::new(wpass_instance, &merged_args);
    runner.handle(&files).await.unwrap();
    runner.join().await.unwrap();
    if merged_args.format {
        hack::format_password_file(&merged_args.password_file).unwrap();
    }
    if merged_args.debug > 0 {
        // wait for enter key press
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
    Ok(())
}

fn check_args(options: &CmdArgument) -> Result<()> {
    if options.file_path.is_dir() {
        if !options.recursive {
            return Err(anyhow!(
                "Directory detected, use -r to extract all files in the directory"
            ));
        }
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
    let args_merged: CmdArgumentMerged = CmdArgumentMerged {
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
        output: options.output,
        local: options.local,
        new_directory: options.new_directory,
        debug: options.debug,
        delete: options.delete,
        format: options.format,
        max_thread: if options.jobs == 0 {
            16
        } else {
            options.jobs
        },
    };

    debug!("After initialization: {:?}", args_merged);
    let wpass = WPassInstance::new(
        get_password(&args_merged.password_file).unwrap(),
        args_merged.executable_path.clone(),
    );
    Ok((args_merged, wpass))
}
