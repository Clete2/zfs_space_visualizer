use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

const GITHUB_API_URL: &str = "https://api.github.com/repos/Clete2/zfs_space_visualizer/releases/latest";

pub async fn check_and_update() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version);

    let latest_release = fetch_latest_release().await?;
    let latest_version = latest_release.tag_name.strip_prefix('v').unwrap_or(&latest_release.tag_name);

    println!("Latest version: {}", latest_version);

    if current_version == latest_version {
        println!("Already running the latest version!");
        return Ok(());
    }

    println!("New version available: {} -> {}", current_version, latest_version);

    let asset_name = get_asset_name_for_platform()?;
    let asset = latest_release.assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow!("No asset found for current platform: {}", asset_name))?;

    println!("Downloading update from: {}", asset.browser_download_url);
    let binary_data = download_binary(&asset.browser_download_url).await?;

    println!("Replacing binary...");
    replace_current_binary(&binary_data)?;

    println!("Update complete! Restarting...");
    restart_application()?;

    Ok(())
}

async fn fetch_latest_release() -> Result<GitHubRelease> {
    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_API_URL)
        .header("User-Agent", "zfs_space_visualizer")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("GitHub API request failed: {}", response.status()));
    }

    let release: GitHubRelease = response.json().await?;
    Ok(release)
}

async fn download_binary(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "zfs_space_visualizer")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed: {}", response.status()));
    }

    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

fn replace_current_binary(binary_data: &[u8]) -> Result<()> {
    let current_exe = env::current_exe()?;

    // Create a temporary file in the same directory as the current executable
    let temp_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory of current executable"))?;

    let mut temp_file = NamedTempFile::new_in(temp_dir)?;

    // Write the new binary data to the temp file
    temp_file.write_all(binary_data)?;
    temp_file.flush()?;

    // Make the temp file executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = temp_file.as_file().metadata()?.permissions();
        perms.set_mode(0o755);
        temp_file.as_file().set_permissions(perms)?;
    }

    // Atomically replace the current binary
    let temp_path = temp_file.into_temp_path();
    temp_path.persist(&current_exe)?;

    Ok(())
}

fn restart_application() -> Result<()> {
    let current_exe = env::current_exe()?;
    let args: Vec<String> = env::args().collect();

    // Remove the "update" command from args if present
    let filtered_args: Vec<&str> = args.iter()
        .skip(1) // Skip the binary name
        .map(|s| s.as_str())
        .filter(|&arg| arg != "update")
        .collect();

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = std::process::Command::new(current_exe)
            .args(filtered_args)
            .exec();
    }

    #[cfg(not(unix))]
    {
        std::process::Command::new(current_exe)
            .args(filtered_args)
            .spawn()?;
        std::process::exit(0);
    }

    Ok(())
}

fn get_asset_name_for_platform() -> Result<String> {
    let arch = env::consts::ARCH;
    let os = env::consts::OS;

    match (arch, os) {
        ("x86_64", "linux") => Ok("zfs_space_visualizer-x86_64-unknown-linux-musl".to_string()),
        ("aarch64", "linux") => Ok("zfs_space_visualizer-aarch64-unknown-linux-musl".to_string()),
        ("x86_64", "macos") => Ok("zfs_space_visualizer-x86_64-apple-darwin".to_string()),
        ("aarch64", "macos") => Ok("zfs_space_visualizer-aarch64-apple-darwin".to_string()),
        _ => Err(anyhow!("Unsupported platform: {}-{}", arch, os)),
    }
}