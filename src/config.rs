use crate::timestamp::Range;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_TOML: &str = r#"# Automatically generated config
[archive]
prefix = ".rattlebeaver."
timestamp_format = "%Y-%m-%d_%H-%M-%S"

[ranges]
latest = 10

[ranges.minutes]
total = 3
allow_sparse = true
include_first = true
include_last = true

[ranges.hours]
total = 5
allow_sparse = true
include_first = true
include_last = true

[ranges.days]
total = 10
allow_sparse = true
include_first = true
include_last = true

[ranges.months]
total = 12
allow_sparse = true
include_first = true
include_last = true

[ranges.years]
total = 10
allow_sparse = true
include_first = true
include_last = true
"#;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub archive: Archive,
    pub ranges: Ranges,
}

impl Config {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let s = std::fs::read_to_string(path).context("read config file")?;
        toml::from_str(&s).context("decode config toml")
    }

    pub fn from_toml(toml_str: impl AsRef<str>) -> Result<Self> {
        Ok(toml::from_str(toml_str.as_ref())?)
    }

    pub fn as_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(&self)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG_TOML).expect("builtin default toml")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Archive {
    pub prefix: String,
    pub timestamp_format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ranges {
    pub latest: usize,
    pub minutes: RollingRange,
    pub hours: RollingRange,
    pub days: RollingRange,
    pub months: RollingRange,
    pub years: RollingRange,
}

impl Ranges {
    #[must_use]
    pub fn iter_ranges(&self) -> [(Range, &RollingRange); 5] {
        [
            (Range::Minute, &self.minutes),
            (Range::Hour, &self.hours),
            (Range::Day, &self.days),
            (Range::Month, &self.months),
            (Range::Year, &self.years),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollingRange {
    pub total: usize,
    pub allow_sparse: bool,
    pub include_first: bool,
    pub include_last: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        Config::default();
    }
}
