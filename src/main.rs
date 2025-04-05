use anyhow::{Context, Result};
use clap::Parser;
use rattlebeaver::{ArchiveMode, Config, Entry, TimestampSelection, create_backup, read_backups};
use std::path::{Path, PathBuf};

#[allow(clippy::doc_markdown)]
#[derive(Debug, Parser)]
struct Args {
    /// Directory for saved backups [defaults to RATTLEBEAVER_TARGET_DIR from environment]
    #[arg(short = 't', long)]
    target_dir: Option<PathBuf>,
    /// Path to config file [defaults to TARGET_DIR/rattlebeaver.config.toml]
    #[arg(long)]
    config: Option<PathBuf>,
    /// Subcommand
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Parser, Clone)]
enum Command {
    /// Add new backups
    Add(ArgsAdd),
    /// List existing backups
    List(ArgsList),
    /// Delete stale backups
    Delete(ArgsDelete),
    /// Print debug info
    Debug,
}

#[derive(Debug, Parser, Clone)]
struct ArgsAdd {
    /// Files or directories to add
    #[arg()]
    files: Vec<PathBuf>,
    /// How to select the timestamp for the backups
    #[arg(short = 't', long, default_value = "file-created")]
    timestamp: TimestampSelection,
    /// How to handle single files
    #[arg(short = 'm', long, default_value = "auto-detect")]
    archive_mode: ArchiveMode,
    /// Don't stop on first failure
    #[arg(short = 'f', long)]
    force: bool,
    /// Also delete stale backups
    #[arg(short = 'D', long)]
    delete: bool,
}

#[derive(Debug, Parser, Clone)]
struct ArgsList {
    /// Show all details
    #[arg(short = 'a', long)]
    all: bool,
    /// Select details to show
    #[arg(last = true)]
    details: Vec<ListingDetails>,
}

#[derive(Debug, Parser, Clone)]
struct ArgsDelete {
    /// Actually delete
    #[arg(short = 'x', long)]
    execute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ListingDetails {
    Time,
    Name,
    Size,
    Fulfills,
}

impl ListingDetails {
    fn all() -> Vec<Self> {
        vec![Self::Time, Self::Size, Self::Name, Self::Fulfills]
    }

    fn default_list() -> Vec<Self> {
        vec![Self::Time, Self::Name]
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let target_dir = if let Some(target_dir) = args.target_dir {
        target_dir
    } else {
        let target_dir = std::env::var("RATTLEBEAVER_TARGET_DIR")
            .context("missing RATTLEBEAVER_TARGET_DIR from environment or from CLI args")?;
        PathBuf::from(target_dir)
    };
    std::fs::create_dir_all(&target_dir).context("create target directory")?;

    let config_path = args
        .config
        .clone()
        .unwrap_or(target_dir.join("rattlebeaver.config.toml"));
    generate_missing_config(&config_path).context("generate new default config")?;
    let config = Config::from_path(&config_path).context("load config")?;

    match args.command {
        Command::Add(subargs) => {
            if subargs.files.is_empty() {
                anyhow::bail!("No files selected to back up.");
            }
            let mut errors = Vec::new();
            for file in subargs.files {
                let new_backup_result = create_backup(
                    &file,
                    &target_dir,
                    &config.archive,
                    subargs.timestamp,
                    subargs.archive_mode,
                )
                .with_context(|| format!("backup file: {file:?}"));
                match new_backup_result {
                    Ok(new_backup) => println!("{}", new_backup.display()),
                    Err(error) => {
                        if subargs.force {
                            errors.push(error);
                        } else {
                            return Err(error);
                        }
                    }
                }
            }
            if !errors.is_empty() {
                for error in &errors {
                    eprintln!("Encountered error: {error:?}");
                }
            }
            if let Some(error) = errors.into_iter().next() {
                return Err(error);
            }
            if subargs.delete {
                delete_stale(&target_dir, &config, true).context("delete stale backups")?;
            }
        }
        Command::List(subargs) => {
            let details = if subargs.all {
                ListingDetails::all()
            } else if subargs.details.is_empty() {
                ListingDetails::default_list()
            } else {
                subargs.details
            };
            list(&target_dir, &config, &details).context("list backups")?;
        }
        Command::Delete(subargs) => {
            delete_stale(&target_dir, &config, subargs.execute).context("delete stale backups")?;
        }
        Command::Debug => {
            println!("Target dir: {}", target_dir.display());
            println!("Config file path: {}", config_path.display());
            println!("{config:#?}");
        }
    }
    Ok(())
}

fn generate_missing_config(config_file: impl AsRef<Path>) -> Result<()> {
    if config_file.as_ref().exists() {
        return Ok(());
    }
    eprintln!("Writing new config at {}", config_file.as_ref().display());
    let default_toml = Config::default()
        .as_toml()
        .context("encode default config toml")?;
    std::fs::write(&config_file, default_toml).context("write default config file")?;
    Ok(())
}

fn delete_stale(target: &Path, config: &Config, execute: bool) -> Result<()> {
    let delete_backups: Vec<Entry> = read_backups(target, config)
        .context("read backups")?
        .into_iter()
        .filter(|b| b.fulfills.is_empty())
        .collect();
    if delete_backups.is_empty() {
        eprintln!("No stale backups.");
        return Ok(());
    }
    if execute {
        eprintln!("Deleting:");
    } else {
        eprintln!("Would delete:");
    }
    for b in delete_backups {
        println!("{}", b.path.display());
        if execute {
            std::fs::remove_file(&b.path).with_context(|| format!("delete {}", b.path.display()))?;
        }
    }
    Ok(())
}

fn list(target: &Path, config: &Config, details: &[ListingDetails]) -> Result<()> {
    let all_backups = read_backups(target, config).context("read backups")?;
    for backup in &all_backups {
        let mut display_strings = Vec::new();
        for desired in details {
            let display = match desired {
                ListingDetails::Name => backup.path.display().to_string(),
                ListingDetails::Time => backup.timestamp.humanized(),
                ListingDetails::Fulfills => backup.fulfills.join(" :: "),
                ListingDetails::Size => {
                    let file_size_bytes = backup.metadata().context("get file metadata")?.len();
                    format!("{file_size_bytes} bytes")
                }
            };
            display_strings.push(display);
        }
        println!("{}", display_strings.join(" | "));
    }
    Ok(())
}
