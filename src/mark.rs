use crate::config;
use crate::entry::{Entry, Fulfillment, read_dir};
use crate::timestamp::{Range, Timestamp};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn read_backups(target: &Path, config: &config::Config) -> Result<Vec<Entry>> {
    let mut all_backups = read_dir(target, &config.archive)?;
    // Mark latest
    all_backups
        .iter_mut()
        .rev()
        .take(config.ranges.latest)
        .enumerate()
        .for_each(|(i, b)| {
            b.fulfills.push(Fulfillment {
                range: None,
                index: i + 1,
                first_or_last: true,
            });
        });
    let mut all_backups: HashMap<Timestamp, Entry> =
        all_backups.into_iter().map(|b| (b.timestamp, b)).collect();
    let now = Timestamp::now();
    for (range, range_config) in config.ranges.iter_ranges() {
        mark_range(&mut all_backups, now, range, range_config)
            .with_context(|| format!("{range:?}"))?;
    }
    let mut final_backups: Vec<Entry> = all_backups.into_values().collect();
    final_backups.sort();
    Ok(final_backups)
}

fn mark_range(
    all_backups: &mut HashMap<Timestamp, Entry>,
    now: Timestamp,
    range: Range,
    config: &config::RollingRange,
) -> Result<()> {
    // Create all buckets
    let mut bucket_timestamps: Vec<Timestamp> = Vec::new();
    if config.allow_sparse {
        let mut all_backup_floors: Vec<Timestamp> = all_backups
            .keys()
            .map(|ts| ts.floor(range))
            .collect::<HashSet<Timestamp>>()
            .into_iter()
            .collect();
        all_backup_floors.sort();
        bucket_timestamps.extend(all_backup_floors.into_iter().rev().take(config.total));
    } else {
        for shift_amount in 0..config.total {
            let shift_amount = i32::try_from(shift_amount)
                .with_context(|| format!("shifting by {shift_amount}"))?;
            let ts = now.floor(range).shift(range, -shift_amount);
            bucket_timestamps.push(ts);
        }
    }
    let mut buckets = Buckets::new(bucket_timestamps);
    // Place backups in buckets
    for backup in all_backups.values() {
        let backup_floored = backup.timestamp.floor(range);
        let _found_bucket = buckets.push(backup_floored, backup.timestamp);
    }
    // Sort buckets and take first/last
    for (i, (_bucket_timestamp, backup_timestamps)) in buckets.sorted().iter_mut().enumerate() {
        if config.include_first {
            if let Some(first_backup_timestamp) = backup_timestamps.first_mut() {
                let Some(original) = all_backups.get_mut(first_backup_timestamp) else {
                    anyhow::bail!("{first_backup_timestamp} not found in original list");
                };
                original.fulfills.push(Fulfillment {
                    range: Some(range),
                    index: i + 1,
                    first_or_last: true,
                });
            }
        }
        if config.include_last {
            if let Some(last_backup_timestamp) = backup_timestamps.last_mut() {
                let Some(original) = all_backups.get_mut(last_backup_timestamp) else {
                    anyhow::bail!("{last_backup_timestamp} not found in original list");
                };
                original.fulfills.push(Fulfillment {
                    range: Some(range),
                    index: i + 1,
                    first_or_last: false,
                });
            }
        }
    }
    Ok(())
}

struct Buckets(HashMap<Timestamp, Vec<Timestamp>>);

impl Buckets {
    fn new(buckets: Vec<Timestamp>) -> Self {
        let buckets = buckets.into_iter().map(|b| (b, Vec::new())).collect();
        Self(buckets)
    }

    fn push(&mut self, bucket: Timestamp, value: Timestamp) -> bool {
        let Some(vec) = self.0.get_mut(&bucket) else {
            return false;
        };
        vec.push(value);
        true
    }

    fn sorted(&self) -> Vec<(Timestamp, Vec<Timestamp>)> {
        let mut order: Vec<Timestamp> = self.0.keys().copied().collect();
        order.sort();
        order.reverse();
        order
            .iter()
            .copied()
            .map(|t| {
                let mut entries = self.0.get(&t).expect("key from order").to_owned();
                entries.sort();
                (t, entries)
            })
            .collect()
    }
}
