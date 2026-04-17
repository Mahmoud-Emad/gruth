//! Self-update — check for new releases and replace the current binary.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::Path;

const GITHUB_REPO: &str = "Mahmoud-Emad/gruth";
const API_URL: &str = "https://api.github.com/repos/Mahmoud-Emad/gruth/releases/latest";

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn commit_hash() -> &'static str {
    env!("GIT_HASH")
}

pub fn version_string() -> String {
    format!("gruth {} ({})", current_version(), commit_hash())
}

/// Fetch the latest release tag from GitHub. Returns the tag name (e.g., "v0.3.0").
pub fn check_latest_version() -> Result<String> {
    let resp: serde_json::Value = ureq::get(API_URL)
        .set("User-Agent", "gruth")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to reach GitHub API")?
        .into_json()
        .context("Failed to parse GitHub API response")?;

    let tag = resp["tag_name"]
        .as_str()
        .context("No tag_name in release")?
        .to_string();

    Ok(tag)
}

/// Compare semver strings. Returns true if `latest` is newer than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    let latest_clean = latest.strip_prefix('v').unwrap_or(latest);
    let current_clean = current.strip_prefix('v').unwrap_or(current);

    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };

    let cur = parse(current_clean);
    let lat = parse(latest_clean);

    lat > cur
}

/// Detect the correct asset name for this platform.
fn asset_name() -> Result<&'static str> {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("gruth-macos-arm64.tar.gz")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("gruth-linux-amd64.tar.gz")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("gruth-linux-arm64.tar.gz")
    } else {
        bail!(
            "No prebuilt binary for {}/{}. Build from source instead.",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}

fn log_step(msg: &str) {
    println!("  \x1b[36m→\x1b[0m {}", msg);
}

fn log_ok(msg: &str) {
    println!("  \x1b[32m✓\x1b[0m {}", msg);
}

fn log_info(msg: &str) {
    println!("  \x1b[90m  {}\x1b[0m", msg);
}

/// Check if we can write to the binary's directory.
fn needs_sudo(path: &Path) -> bool {
    let dir = path.parent().unwrap_or(path);
    let test_file = dir.join(".gruth-write-test");
    match std::fs::write(&test_file, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            false
        }
        Err(_) => true,
    }
}

/// Re-run the update via sudo.
fn run_with_sudo(tmp_binary: &Path, target: &Path) -> Result<()> {
    log_step("Elevated permissions required — running sudo...");
    log_info(&format!("sudo cp {} {}", tmp_binary.display(), target.display()));
    println!();

    let status = std::process::Command::new("sudo")
        .args(["cp", &tmp_binary.to_string_lossy(), &target.to_string_lossy()])
        .status()
        .context("Failed to run sudo")?;

    if !status.success() {
        bail!("sudo cp failed with exit code {}", status.code().unwrap_or(-1));
    }

    // Set permissions
    let _ = std::process::Command::new("sudo")
        .args(["chmod", "+x", &target.to_string_lossy()])
        .status();

    Ok(())
}

/// Download the latest release and replace the current binary.
pub fn run_update() -> Result<()> {
    println!();
    println!("  \x1b[1mgruth self-update\x1b[0m");
    println!("  \x1b[90m─────────────────────────────────────\x1b[0m");
    println!();

    // Current version
    log_info(&format!("Current: {}", version_string()));
    println!();

    // Check latest version
    log_step("Checking GitHub for latest release...");
    let latest = check_latest_version()?;
    let latest_clean = latest.strip_prefix('v').unwrap_or(&latest);

    if !is_newer(current_version(), &latest) {
        log_ok(&format!("Already up to date ({})", latest));
        println!();
        return Ok(());
    }

    log_ok(&format!(
        "New version available: {} → {}",
        current_version(),
        latest_clean
    ));
    println!();

    // Determine asset
    let asset = asset_name()?;
    let download_url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, latest, asset
    );

    // Download
    log_step(&format!("Downloading {}...", asset));
    log_info(&download_url);

    let resp = ureq::get(&download_url)
        .call()
        .context("Failed to download release")?;

    let mut archive_bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut archive_bytes)
        .context("Failed to read download")?;

    log_ok(&format!(
        "Downloaded {:.1} MB",
        archive_bytes.len() as f64 / 1_048_576.0
    ));
    println!();

    // Extract tar.gz
    log_step("Extracting binary from archive...");
    let binary_data = extract_tar_gz(&archive_bytes)?;
    log_ok(&format!(
        "Extracted binary ({:.1} MB)",
        binary_data.len() as f64 / 1_048_576.0
    ));
    println!();

    // Determine target path
    let current_exe = std::env::current_exe().context("Cannot determine current executable path")?;
    let current_exe = current_exe
        .canonicalize()
        .unwrap_or(current_exe);

    log_step(&format!("Installing to {}...", current_exe.display()));

    if needs_sudo(&current_exe) {
        // Write to temp file, then sudo cp
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join("gruth-update-tmp");

        std::fs::write(&tmp_path, &binary_data)
            .context("Failed to write temporary binary")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755));
        }

        run_with_sudo(&tmp_path, &current_exe)?;

        let _ = std::fs::remove_file(&tmp_path);
    } else {
        // Direct replace: backup → write → cleanup
        let backup = current_exe.with_extension("old");

        std::fs::rename(&current_exe, &backup)
            .context("Failed to create backup of current binary")?;

        if let Err(e) = std::fs::write(&current_exe, &binary_data) {
            let _ = std::fs::rename(&backup, &current_exe);
            bail!("Failed to write new binary: {}", e);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
        }

        let _ = std::fs::remove_file(&backup);
    }

    log_ok("Binary replaced successfully");
    println!();

    // Verify
    log_step("Verifying installation...");
    match std::process::Command::new(&current_exe).arg("version").output() {
        Ok(output) => {
            let version_out = String::from_utf8_lossy(&output.stdout).trim().to_string();
            log_ok(&format!("Installed: {}", version_out));
        }
        Err(_) => {
            log_ok(&format!("Updated to {}", latest_clean));
        }
    }

    println!();
    println!("  \x1b[90m─────────────────────────────────────\x1b[0m");
    println!(
        "  \x1b[32m✓ gruth updated to {} successfully!\x1b[0m",
        latest_clean
    );
    println!();

    Ok(())
}

/// Extract a single file named "gruth" from a tar.gz archive in memory.
fn extract_tar_gz(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut tar_data = Vec::new();
    decoder
        .read_to_end(&mut tar_data)
        .context("Failed to decompress gzip")?;

    let mut pos = 0;
    while pos + 512 <= tar_data.len() {
        let header = &tar_data[pos..pos + 512];

        if header.iter().all(|&b| b == 0) {
            break;
        }

        let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end]).unwrap_or("");

        let size_str = std::str::from_utf8(&header[124..136])
            .unwrap_or("0")
            .trim_matches(|c: char| c == '\0' || c == ' ');
        let size = usize::from_str_radix(size_str, 8).unwrap_or(0);

        let data_start = pos + 512;
        let data_end = data_start + size;

        let file_name = name.rsplit('/').next().unwrap_or(name);
        if file_name == "gruth" && size > 0 && data_end <= tar_data.len() {
            return Ok(tar_data[data_start..data_end].to_vec());
        }

        let blocks = (size + 511) / 512;
        pos = data_start + blocks * 512;
    }

    bail!("Binary 'gruth' not found in archive")
}

/// Print version info.
pub fn print_version() {
    println!("{}", version_string());
}
