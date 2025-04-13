use crate::config;
use crate::entry::read_dir;
use crate::timestamp::Timestamp;
use anyhow::{Context, Result};
use chrono::{Local, Timelike};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ArchiveMode {
    /// Tarball and compress if not already
    AutoDetect,
    /// Use file as-is
    AsIs,
    /// Tarball and compress always
    Force,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TimestampSelection {
    Now,
    FileCreated,
    FileModified,
}

#[derive(Debug)]
pub enum BackupError {
    TimestampConflict(String),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for BackupError {
    fn from(value: anyhow::Error) -> Self {
        BackupError::Other(value)
    }
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::TimestampConflict(s) => s.to_owned(),
            Self::Other(e) => e.to_string(),
        };
        write!(f, "{display}")
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Other(anyhow_error) = self {
            Some(&**anyhow_error)
        } else {
            None
        }
    }
}

pub fn create_backup(
    source: &Path,
    target: &Path,
    config: &config::Archive,
    timestamp: TimestampSelection,
    archive_behavior: ArchiveMode,
) -> std::result::Result<PathBuf, BackupError> {
    ensure_dir(target)?;
    let timestamp = get_file_timestamp(source, timestamp)?;
    let existing_backups = read_dir(target, config).context("read existing backups")?;
    for existing in existing_backups {
        if timestamp == existing.timestamp {
            let error = BackupError::TimestampConflict(format!(
                "timestamp {timestamp} conflicts with existing backup: {}",
                existing.path.display()
            ));
            return Err(error);
        }
    }
    let file_name = format!(
        "{}{}",
        config.prefix,
        timestamp.as_ref().format(&config.timestamp_format),
    );

    let final_target_path = if source.is_dir() {
        let source_stem = get_file_stem(source)?;
        let target_path = target.join(format!("{file_name}.{source_stem}.tar.gz"));
        let tar_gz = File::create(&target_path).context("create archive file")?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tarball = tar::Builder::new(enc);
        tarball
            .append_dir_all("", source)
            .context("add dir to tarball")?;
        tarball.finish().context("create tarball")?;
        target_path
    } else if source.is_file() {
        let is_archive = source.display().to_string().ends_with(".tar.gz");
        let make_archive = match (archive_behavior, is_archive) {
            (ArchiveMode::Force, _) | (ArchiveMode::AutoDetect, false) => true,
            (ArchiveMode::AsIs, _) | (ArchiveMode::AutoDetect, true) => false,
        };
        if make_archive {
            let source_stem = get_file_stem(source)?;
            let mut source_file = std::fs::File::open(source).context("open source file")?;
            let target_path = target.join(format!("{file_name}.{source_stem}.tar.gz"));
            let tar_gz = File::create(&target_path).context("create archive file")?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tarball = tar::Builder::new(enc);
            tarball
                .append_file(
                    source.file_name().context("missing file name")?,
                    &mut source_file,
                )
                .context("add dir to tarball")?;
            tarball.finish().context("create tarball")?;
            target_path
        } else {
            let source_name = source
                .file_name()
                .context("get file name")?
                .to_string_lossy();
            let target_path = target.join(format!("{file_name}.{source_name}"));
            std::fs::copy(source, &target_path).context("copy file")?;
            target_path
        }
    } else {
        return Err(anyhow::anyhow!("source file is neither a file nor directory").into());
    };

    Ok(final_target_path)
}

fn get_file_timestamp(file: &Path, selection: TimestampSelection) -> Result<Timestamp> {
    let timestamp = match selection {
        TimestampSelection::Now => Local::now(),
        TimestampSelection::FileCreated => {
            let metadata = file.metadata().context("get file metadata")?;
            metadata.created().context("get file created time")?.into()
        }
        TimestampSelection::FileModified => {
            let metadata = file.metadata().context("get file metadata")?;
            metadata
                .modified()
                .context("get file modified time")?
                .into()
        }
    };
    let timestamp = timestamp.with_nanosecond(0).context("zero nanoseconds")?;
    Ok(Timestamp(timestamp))
}

fn get_file_stem(source: &Path) -> Result<String> {
    Ok(source
        .file_stem()
        .context("get file stem")?
        .to_string_lossy()
        .to_string())
}

fn ensure_dir(target: &Path) -> Result<()> {
    if !target.exists() {
        std::fs::create_dir_all(target).context("create target dir")?;
    } else if !target.is_dir() {
        anyhow::bail!("{target:?} is not a directory");
    }
    Ok(())
}
