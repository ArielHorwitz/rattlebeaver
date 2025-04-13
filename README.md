# RattleBeaver

Manage rolling backups for frequently generated backups.

## Why

When managing frequent backups, keeping the last X backups is not enough: reducing X quickly loses older yet still helpful backups, and increasing X produces many backups that are not helpful. `rattlebeaver` allows to keep a sensible amount of backups for several timespans. Some example configurations:
* first and last backup for every hour of the last 5 hours and the last backup for every day of the last 20 days.
* one backup for every minute of the last 60 minutes, and one backup for every hour of the last 1000 hours, as well as first and last backup for every month of the last 12 months.

## How to use

Install using:
```
cargo install rattlebeaver
```

Every command in rattlebeaver will require specifying the `TARGET_DIR` - the directory containing the rolling backups. This can be done using `-t <TARGET_DIR>` or setting the `RATTLEBEAVER_TARGET_DIR` environment variable.

A default configuration file will be generated inside the target dir as `<TARGET_DIR>/rattlebeaver.config.toml`. This config will determine which backups are relevant and which are stale and need to be deleted.

To add a new rolling backup:
```
rattlebeaver add path/to/file-or-dir
```

To see existing backups:
```
rattlebeaver list
```

To delete stale backups:
```
rattlebeaver delete --execute
```

## Config

> To understand how rattlebeaver determines which backups are stale, run `rattlebeaver list -a` to see what every backup entry fulfills according to the config. Entries that don't fulfill anything are considered stale and will be deleted by the `rattlebeaver delete` command.

The `ranges.latest` determines how many of the last X backups to keep.

For the specific ranges (e.g. `ranges.days`):
* `total` determines how many instances to consider for that range (e.g. 3 days)
* `allow_sparse` determines whether the total includes empty instances (e.g. last 3 days that have backups or the last 3 days of the calendar)
* `include_first` determines if the first backup of every instance should be kept
* `include_last` determines if the last backup of every instance should be kept
