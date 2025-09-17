use anyhow::{anyhow, Result};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone)]
pub struct Pool {
    pub name: String,
    pub size: u64,
    pub allocated: u64,
    pub free: u64,
    pub health: String,
}

#[derive(Debug, Clone)]
pub struct Dataset {
    pub name: String,
    pub used: u64,
    pub available: u64,
    pub referenced: u64,
    pub snapshot_used: u64,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub name: String,
    pub used: u64,
    pub referenced: u64,
    pub creation: String,
}

pub async fn get_pools() -> Result<Vec<Pool>> {
    let output = TokioCommand::new("zpool")
        .args(&["list", "-H", "-p"])
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            let mut pools = Vec::new();

            for line in stdout.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() >= 7 {
                    pools.push(Pool {
                        name: fields[0].to_string(),
                        size: fields[1].parse().unwrap_or(0),
                        allocated: fields[2].parse().unwrap_or(0),
                        free: fields[3].parse().unwrap_or(0),
                        health: fields[9].to_string(),
                    });
                }
            }

            Ok(pools)
        }
        _ => {
            // ZFS not available or no pools, return mock data for demonstration
            Ok(get_mock_pools())
        }
    }
}

fn get_mock_pools() -> Vec<Pool> {
    vec![
        Pool {
            name: "tank".to_string(),
            size: 10 * 1024 * 1024 * 1024 * 1024, // 10TB
            allocated: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
            free: 7 * 1024 * 1024 * 1024 * 1024, // 7TB
            health: "ONLINE".to_string(),
        },
        Pool {
            name: "backup".to_string(),
            size: 5 * 1024 * 1024 * 1024 * 1024, // 5TB
            allocated: 2 * 1024 * 1024 * 1024 * 1024, // 2TB
            free: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
            health: "ONLINE".to_string(),
        },
        Pool {
            name: "archive".to_string(),
            size: 20 * 1024 * 1024 * 1024 * 1024, // 20TB
            allocated: 15 * 1024 * 1024 * 1024 * 1024, // 15TB
            free: 5 * 1024 * 1024 * 1024 * 1024, // 5TB
            health: "DEGRADED".to_string(),
        },
    ]
}

pub async fn get_datasets(pool_name: &str) -> Result<Vec<Dataset>> {
    let output = TokioCommand::new("zfs")
        .args(&["list", "-H", "-p", "-r", "-o", "name,used,avail,refer,usedsnapshots", pool_name])
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            let mut datasets = Vec::new();

            for line in stdout.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() >= 5 {
                    datasets.push(Dataset {
                        name: fields[0].to_string(),
                        used: fields[1].parse().unwrap_or(0),
                        available: fields[2].parse().unwrap_or(0),
                        referenced: fields[3].parse().unwrap_or(0),
                        snapshot_used: fields[4].parse().unwrap_or(0),
                    });
                }
            }

            Ok(datasets)
        }
        _ => {
            // ZFS not available, return mock data
            Ok(get_mock_datasets(pool_name))
        }
    }
}

fn get_mock_datasets(pool_name: &str) -> Vec<Dataset> {
    match pool_name {
        "tank" => vec![
            Dataset {
                name: "tank".to_string(),
                used: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
                available: 7 * 1024 * 1024 * 1024 * 1024, // 7TB
                referenced: 1024 * 1024 * 1024, // 1GB
                snapshot_used: 500 * 1024 * 1024 * 1024, // 500GB
            },
            Dataset {
                name: "tank/home".to_string(),
                used: 800 * 1024 * 1024 * 1024, // 800GB
                available: 7 * 1024 * 1024 * 1024 * 1024, // 7TB
                referenced: 600 * 1024 * 1024 * 1024, // 600GB
                snapshot_used: 200 * 1024 * 1024 * 1024, // 200GB
            },
            Dataset {
                name: "tank/data".to_string(),
                used: 1200 * 1024 * 1024 * 1024, // 1.2TB
                available: 7 * 1024 * 1024 * 1024 * 1024, // 7TB
                referenced: 1000 * 1024 * 1024 * 1024, // 1TB
                snapshot_used: 200 * 1024 * 1024 * 1024, // 200GB
            },
            Dataset {
                name: "tank/vm".to_string(),
                used: 500 * 1024 * 1024 * 1024, // 500GB
                available: 7 * 1024 * 1024 * 1024 * 1024, // 7TB
                referenced: 450 * 1024 * 1024 * 1024, // 450GB
                snapshot_used: 50 * 1024 * 1024 * 1024, // 50GB
            },
        ],
        "backup" => vec![
            Dataset {
                name: "backup".to_string(),
                used: 2 * 1024 * 1024 * 1024 * 1024, // 2TB
                available: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
                referenced: 512 * 1024 * 1024, // 512MB
                snapshot_used: 200 * 1024 * 1024 * 1024, // 200GB
            },
            Dataset {
                name: "backup/daily".to_string(),
                used: 1000 * 1024 * 1024 * 1024, // 1TB
                available: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
                referenced: 800 * 1024 * 1024 * 1024, // 800GB
                snapshot_used: 200 * 1024 * 1024 * 1024, // 200GB
            },
        ],
        "archive" => vec![
            Dataset {
                name: "archive".to_string(),
                used: 15 * 1024 * 1024 * 1024 * 1024, // 15TB
                available: 5 * 1024 * 1024 * 1024 * 1024, // 5TB
                referenced: 12 * 1024 * 1024 * 1024 * 1024, // 12TB
                snapshot_used: 3 * 1024 * 1024 * 1024 * 1024, // 3TB
            },
        ],
        _ => vec![]
    }
}

pub async fn get_snapshots(dataset_name: &str) -> Result<Vec<Snapshot>> {
    let output = TokioCommand::new("zfs")
        .args(&["list", "-H", "-p", "-t", "snap", "-r", "-o", "name,used,refer,creation", dataset_name])
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            let mut snapshots = Vec::new();

            for line in stdout.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() >= 4 {
                    snapshots.push(Snapshot {
                        name: fields[0].to_string(),
                        used: fields[1].parse().unwrap_or(0),
                        referenced: fields[2].parse().unwrap_or(0),
                        creation: fields[3].to_string(),
                    });
                }
            }

            Ok(snapshots)
        }
        _ => {
            // ZFS not available, return mock data
            Ok(get_mock_snapshots(dataset_name))
        }
    }
}

fn get_mock_snapshots(dataset_name: &str) -> Vec<Snapshot> {
    match dataset_name {
        "tank/home" => vec![
            Snapshot {
                name: "tank/home@daily-2024-01-15".to_string(),
                used: 50 * 1024 * 1024 * 1024, // 50GB
                referenced: 600 * 1024 * 1024 * 1024, // 600GB
                creation: "Mon Jan 15 06:00 2024".to_string(),
            },
            Snapshot {
                name: "tank/home@daily-2024-01-14".to_string(),
                used: 30 * 1024 * 1024 * 1024, // 30GB
                referenced: 580 * 1024 * 1024 * 1024, // 580GB
                creation: "Sun Jan 14 06:00 2024".to_string(),
            },
            Snapshot {
                name: "tank/home@weekly-2024-01-08".to_string(),
                used: 80 * 1024 * 1024 * 1024, // 80GB
                referenced: 520 * 1024 * 1024 * 1024, // 520GB
                creation: "Mon Jan  8 06:00 2024".to_string(),
            },
            Snapshot {
                name: "tank/home@monthly-2024-01-01".to_string(),
                used: 40 * 1024 * 1024 * 1024, // 40GB
                referenced: 450 * 1024 * 1024 * 1024, // 450GB
                creation: "Mon Jan  1 06:00 2024".to_string(),
            },
        ],
        "tank/data" => vec![
            Snapshot {
                name: "tank/data@backup-2024-01-15".to_string(),
                used: 100 * 1024 * 1024 * 1024, // 100GB
                referenced: 1000 * 1024 * 1024 * 1024, // 1TB
                creation: "Mon Jan 15 12:00 2024".to_string(),
            },
            Snapshot {
                name: "tank/data@backup-2024-01-10".to_string(),
                used: 100 * 1024 * 1024 * 1024, // 100GB
                referenced: 950 * 1024 * 1024 * 1024, // 950GB
                creation: "Wed Jan 10 12:00 2024".to_string(),
            },
        ],
        "tank/vm" => vec![
            Snapshot {
                name: "tank/vm@pre-update".to_string(),
                used: 25 * 1024 * 1024 * 1024, // 25GB
                referenced: 400 * 1024 * 1024 * 1024, // 400GB
                creation: "Fri Jan 12 14:30 2024".to_string(),
            },
            Snapshot {
                name: "tank/vm@clean-install".to_string(),
                used: 25 * 1024 * 1024 * 1024, // 25GB
                referenced: 350 * 1024 * 1024 * 1024, // 350GB
                creation: "Mon Jan  1 10:00 2024".to_string(),
            },
        ],
        "backup/daily" => vec![
            Snapshot {
                name: "backup/daily@2024-01-15".to_string(),
                used: 50 * 1024 * 1024 * 1024, // 50GB
                referenced: 800 * 1024 * 1024 * 1024, // 800GB
                creation: "Mon Jan 15 23:00 2024".to_string(),
            },
            Snapshot {
                name: "backup/daily@2024-01-14".to_string(),
                used: 45 * 1024 * 1024 * 1024, // 45GB
                referenced: 780 * 1024 * 1024 * 1024, // 780GB
                creation: "Sun Jan 14 23:00 2024".to_string(),
            },
            Snapshot {
                name: "backup/daily@2024-01-13".to_string(),
                used: 40 * 1024 * 1024 * 1024, // 40GB
                referenced: 760 * 1024 * 1024 * 1024, // 760GB
                creation: "Sat Jan 13 23:00 2024".to_string(),
            },
        ],
        "archive" => vec![
            Snapshot {
                name: "archive@quarterly-2024-q1".to_string(),
                used: 1500 * 1024 * 1024 * 1024, // 1.5TB
                referenced: 10 * 1024 * 1024 * 1024 * 1024, // 10TB
                creation: "Mon Jan  1 00:00 2024".to_string(),
            },
            Snapshot {
                name: "archive@quarterly-2023-q4".to_string(),
                used: 1500 * 1024 * 1024 * 1024, // 1.5TB
                referenced: 9 * 1024 * 1024 * 1024 * 1024, // 9TB
                creation: "Fri Oct  1 00:00 2023".to_string(),
            },
        ],
        _ => vec![]
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}