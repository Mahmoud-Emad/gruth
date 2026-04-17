//! Self-update — check for new releases and replace the current binary.

use anyhow::{bail, Context, Result};

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

/// Download the latest release and replace the current binary.
pub fn run_update() -> Result<()> {
    println!("  {}", version_string());
    println!();

    // Check latest version
    print!("  Checking for updates... ");
    let latest = check_latest_version()?;
    let latest_clean = latest.strip_prefix('v').unwrap_or(&latest);

    if !is_newer(current_version(), &latest) {
        println!("already up to date ({})", latest);
        return Ok(());
    }

    println!("new version available: {}", latest);
    println!();

    // Determine asset
    let asset = asset_name()?;
    let download_url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        GITHUB_REPO, latest, asset
    );

    // Download
    print!("  Downloading {}... ", asset);
    let resp = ureq::get(&download_url)
        .call()
        .context("Failed to download release")?;

    let mut archive_bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut archive_bytes)
        .context("Failed to read download")?;
    println!("done ({:.1} MB)", archive_bytes.len() as f64 / 1_048_576.0);

    // Extract tar.gz
    print!("  Extracting... ");
    let decoder = flate2_extract(&archive_bytes)?;
    println!("done");

    // Replace current binary
    let current_exe = std::env::current_exe().context("Cannot determine current executable path")?;
    let backup = current_exe.with_extension("old");

    print!("  Replacing {}... ", current_exe.display());

    // Move current → backup, write new, remove backup
    std::fs::rename(&current_exe, &backup)
        .context("Failed to backup current binary. Try running with sudo.")?;

    if let Err(e) = std::fs::write(&current_exe, &decoder) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        bail!("Failed to write new binary: {}", e);
    }

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
    }

    let _ = std::fs::remove_file(&backup);

    println!("done");
    println!();
    println!(
        "  \x1b[32m✓ Updated to {} successfully!\x1b[0m",
        latest_clean
    );
    println!();

    Ok(())
}

/// Extract a single file named "gruth" from a tar.gz archive in memory.
fn flate2_extract(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;

    // Decompress gzip
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut tar_data = Vec::new();
    decoder
        .read_to_end(&mut tar_data)
        .context("Failed to decompress gzip")?;

    // Parse tar — find the "gruth" entry
    let mut pos = 0;
    while pos + 512 <= tar_data.len() {
        let header = &tar_data[pos..pos + 512];

        // End of archive (two zero blocks)
        if header.iter().all(|&b| b == 0) {
            break;
        }

        // Extract filename (first 100 bytes, null-terminated)
        let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end]).unwrap_or("");

        // Extract file size from octal field at offset 124, length 12
        let size_str = std::str::from_utf8(&header[124..136])
            .unwrap_or("0")
            .trim_matches(|c: char| c == '\0' || c == ' ');
        let size = usize::from_str_radix(size_str, 8).unwrap_or(0);

        let data_start = pos + 512;
        let data_end = data_start + size;

        // Strip path prefix — look for a file named "gruth"
        let file_name = name.rsplit('/').next().unwrap_or(name);
        if file_name == "gruth" && size > 0 && data_end <= tar_data.len() {
            return Ok(tar_data[data_start..data_end].to_vec());
        }

        // Advance past header + data (rounded up to 512-byte blocks)
        let blocks = (size + 511) / 512;
        pos = data_start + blocks * 512;
    }

    bail!("Binary 'gruth' not found in archive")
}

/// Print version info.
pub fn print_version() {
    println!("{}", version_string());
}
