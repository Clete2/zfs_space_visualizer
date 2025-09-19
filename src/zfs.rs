use anyhow::{anyhow, Context, Result};
use std::str;
use tokio::process::Command;

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
    let output = execute_command("zpool", &["list", "-H", "-p"])
        .await
        .context("Failed to list ZFS pools")?;

    let mut pools = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(pool_result) = parse_pool_line(line).await {
            pools.push(pool_result?);
        }
    }
    Ok(pools)
}

async fn parse_pool_line(line: &str) -> Option<Result<Pool>> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 7 {
        return None;
    }

    let pool_name = fields[0];
    let usable_size = get_pool_usable_size(pool_name)
        .await
        .unwrap_or_else(|_| parse_u64(fields[1]));

    Some(Ok(Pool {
        name: pool_name.to_owned(),
        size: parse_u64(fields[1]),
        allocated: parse_u64(fields[2]),
        free: parse_u64(fields[3]),
        health: fields[9].to_owned(),
        usable_size,
    }))
}

async fn get_pool_usable_size(pool_name: &str) -> Result<u64> {
    let output = execute_command("zfs", &["list", "-H", "-p", "-o", "used,avail", pool_name])
        .await
        .with_context(|| format!("Failed to get usable size for pool {}", pool_name))?;

    let line = output
        .lines()
        .next()
        .ok_or_else(|| anyhow!("No output from zfs list"))?;

    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 2 {
        let used = parse_u64(fields[0]);
        let avail = parse_u64(fields[1]);
        Ok(used + avail)
    } else {
        Err(anyhow!("Invalid zfs list output format"))
    }
}


pub async fn get_datasets(pool_name: &str) -> Result<Vec<Dataset>> {
    let output = execute_command(
        "zfs",
        &["list", "-H", "-p", "-r", "-o", "name,used,avail,refer,usedbysnapshots", pool_name],
    )
    .await
    .with_context(|| format!("Failed to list datasets for pool {}", pool_name))?;

    Ok(output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_dataset_line)
        .collect())
}

fn parse_dataset_line(line: &str) -> Option<Dataset> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 5 {
        Some(Dataset {
            name: fields[0].to_owned(),
            used: parse_u64(fields[1]),
            available: parse_u64(fields[2]),
            referenced: parse_u64(fields[3]),
            snapshot_used: parse_u64(fields[4]),
        })
    } else {
        None
    }
}


pub async fn get_snapshots(dataset_name: &str) -> Result<Vec<Snapshot>> {
    let output = execute_command(
        "zfs",
        &["list", "-H", "-p", "-t", "snap", "-r", "-o", "name,used,refer,creation", dataset_name],
    )
    .await
    .with_context(|| format!("Failed to list snapshots for dataset {}", dataset_name))?;

    Ok(output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_snapshot_line)
        .collect())
}

fn parse_snapshot_line(line: &str) -> Option<Snapshot> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 4 {
        Some(Snapshot {
            name: fields[0].to_owned(),
            used: parse_u64(fields[1]),
            referenced: parse_u64(fields[2]),
            creation: fields[3].to_owned(),
        })
    } else {
        None
    }
}

async fn execute_command(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to execute command: {} {}", command, args.join(" ")))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command failed: {} {}\nStderr: {}",
            command,
            args.join(" "),
            str::from_utf8(&output.stderr).unwrap_or("<invalid UTF-8>")
        ));
    }

    str::from_utf8(&output.stdout)
        .context("Command output is not valid UTF-8")
        .map(|s| s.to_owned())
}

fn parse_u64(s: &str) -> u64 {
    s.parse().unwrap_or(0)
}


pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    const THRESHOLD: f64 = 1024.0;

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}