mod extract;
mod password;

use anyhow::Result;
use clap::Parser;
use config::Config;
use extract::try_extract;
use log::{debug, LevelFilter};
use std::{collections::HashMap, io::Write, path::{PathBuf}, process::exit};

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
    #[clap(short = 'D', long)]
    delete: bool,

    /// Generate reg file for windows context menu. With this option enabled the program will not try to extract file, but you still need to provide an arbitrary file name.
    #[clap(short, long)]
    generate: bool,

    /// Format the password file after everything. Sort passwords and deduplicate them. Enabled by default.
    #[clap(short, long)]
    format: bool,
}

extern "C" {
    pub fn system(command: *const u8);
}

fn main() {
    let mut config = Config::default();
    let mut config_path = std::env::current_exe().unwrap();
    config_path.pop();
    config_path.push("config.toml");
    config
        .merge(config::File::with_name(config_path.to_str().unwrap())).unwrap();
    let config = config
            .try_into::<HashMap<String, String>>()
            .unwrap();
    let mut args: CommandLineArgument = CommandLineArgument::parse();
    if args.debug >= 0 {
        env_logger::Builder::new()
            .filter_level(LevelFilter::Debug)
            .init();
    } else {
        env_logger::init();
    }
    if args.generate {
        generate_reg();
        exit(0);
    }
    initialize(&mut args, &config).expect("Fail to initialize arguments");
    let success = try_extract(&args).expect("Failed to extract");
    if success {
        finalize(&args).expect("Fail to finalize");
    }
    // System("pause")
    if args.debug >= 1 || !success {
        let mut stdout = std::io::stdout();
        stdout.write(b"Press Enter to continue...").unwrap();
        stdout.flush().unwrap();
        std::io::Read::read(&mut std::io::stdin(), &mut [0]).unwrap();
    }
    exit(if success { 0 } else { 1 });
}

fn generate_reg() -> () {
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
    let mut reg_file = std::fs::File::create("wpass.reg").expect("Failed to create reg file");
    reg_file
        .write(reg_file_content.as_bytes())
        .expect("Failed to write reg file");
}

fn finalize(args: &CommandLineArgument) -> Result<()> {
    if args.delete {
        std::fs::remove_file(&args.file_path)?;
    }
    if args.format {
        password::format_password_file(&args.password_file)?;
    }
    Ok(())
}

pub fn initialize(options: &mut CommandLineArgument, config:&HashMap<String, String>) -> Result<()> {
    debug!("Read config: {:?}", config);
    debug!("Before initialization: {:?}", options);
    if options.password_file.to_str() == Some("none") {
        options.password_file = {
            let config_path = PathBuf::from(&config["password_dict"]);
            if config_path.is_absolute() {
                config_path
            } else { 
                let mut path = std::env::current_exe().expect("Cannot get exe path");
                path.pop();
                path.push(config_path);
                path
            }
        }
    }

    if options.executable_path.to_str() == Some("none") {
        options.executable_path = {
            let config_path = PathBuf::from(&config["executable_path"]);
            if config_path.is_absolute() {
                config_path
            } else { 
                let mut path = std::env::current_exe().expect("Cannot get exe path");
                path.pop();
                path.push(config_path);
                path
            }
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
            Some(filename) => {
                options.output.push(filename);
                // Some times the file has no extension, in which case the directory will have the same name as the file itself, in which case the we will fail to create the directory.
                if std::path::Path::exists(&options.output) {
                    options.output.pop();
                    options
                        .output
                        .push(format!("{}_extracted", filename.to_str().unwrap()));
                }
            }
            // What the hell?
            None => options.output.push("foobar"),
        }
    }
    debug!("After initialization: {:?}", options);
    Ok(())
}
