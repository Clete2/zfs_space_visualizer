use anyhow::{anyhow, Result};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone)]
pub struct Pool {
    pub name: String,
    pub size: u64,
    pub allocated: u64,
    pub free: u64,
    pub health: String,
    pub usable_size: u64, // Actual usable space from zfs list (accounts for redundancy)
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
        .await?;

    if !output.status.success() {
        return Err(anyhow!("Failed to execute zpool list: {}",
            String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut pools = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() >= 7 {
            let pool_name = fields[0].to_string();
            let usable_size = get_pool_usable_size(&pool_name).await.unwrap_or(fields[1].parse().unwrap_or(0));

            pools.push(Pool {
                name: pool_name,
                size: fields[1].parse().unwrap_or(0),
                allocated: fields[2].parse().unwrap_or(0),
                free: fields[3].parse().unwrap_or(0),
                health: fields[9].to_string(),
                usable_size,
            });
        }
    }

    Ok(pools)
}

async fn get_pool_usable_size(pool_name: &str) -> Result<u64> {
    let output = TokioCommand::new("zfs")
        .args(&["list", "-H", "-p", "-o", "used,avail", pool_name])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!("Failed to execute zfs list for pool {}: {}",
            pool_name, String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let line = stdout.lines().next().ok_or_else(|| anyhow!("No output from zfs list"))?;

    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 2 {
        let used: u64 = fields[0].parse().unwrap_or(0);
        let avail: u64 = fields[1].parse().unwrap_or(0);
        Ok(used + avail) // Total usable space = used + available
    } else {
        Err(anyhow!("Invalid zfs list output format"))
    }
}


pub async fn get_datasets(pool_name: &str) -> Result<Vec<Dataset>> {
    let output = TokioCommand::new("zfs")
        .args(&["list", "-H", "-p", "-r", "-o", "name,used,avail,refer,usedbysnapshots", pool_name])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!("Failed to execute zfs list: {}",
            String::from_utf8_lossy(&output.stderr)));
    }

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


pub async fn get_snapshots(dataset_name: &str) -> Result<Vec<Snapshot>> {
    let output = TokioCommand::new("zfs")
        .args(&["list", "-H", "-p", "-t", "snap", "-r", "-o", "name,used,refer,creation", dataset_name])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!("Failed to execute zfs list for snapshots: {}",
            String::from_utf8_lossy(&output.stderr)));
    }

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