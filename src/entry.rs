use crate::config;
use crate::timestamp::Timestamp;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub timestamp: Timestamp,
    pub fulfills: Vec<String>,
}

impl Entry {
    pub fn from_path(path: PathBuf, config: &config::Archive) -> Result<Option<Self>> {
        let filename = path
            .file_name()
            .context("no file name")?
            .to_str()
            .context("file name no utf-8")?;
        let Some(removed_prefix) = filename.strip_prefix(config.prefix.as_str()) else {
            return Ok(None);
        };
        let raw_timestamp = removed_prefix
            .split_once('.')
            .map_or(removed_prefix, |o| o.0);
        let timestamp = Timestamp::parse_from_str(raw_timestamp, config.timestamp_format.as_str())
            .context("failed to parse timestamp from filename")?;
        Ok(Some(Self {
            path,
            timestamp,
            fulfills: Vec::new(),
        }))
    }

    pub fn metadata(&self) -> Result<Metadata> {
        Ok(std::fs::metadata(&self.path)?)
    }
}

impl Eq for Entry {}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Entry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.path.as_os_str().as_encoded_bytes());
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.timestamp)
    }
}

pub(crate) fn read_dir(target: &Path, config: &config::Archive) -> Result<Vec<Entry>> {
    let mut all_backups = Vec::new();
    let mut timestamps: HashMap<Timestamp, Entry> = HashMap::new();
    for file in target.read_dir().context("read target directory")? {
        let file = file.context("read file from dir")?;
        let file_name = file.file_name();
        let file_path = file.path();
        let entry_opt = Entry::from_path(file_path, config)
            .with_context(|| format!("parse {}", file_name.to_string_lossy()))?;
        let Some(backup) = entry_opt else {
            continue;
        };
        if let Some(existing) = timestamps.get(&backup.timestamp) {
            anyhow::bail!(
                "timestamps conflict for {} and {}",
                backup.path.display(),
                existing.path.display()
            );
        }
        timestamps.insert(backup.timestamp, backup.clone());
        all_backups.push(backup);
    }
    all_backups.sort();
    Ok(all_backups)
}
