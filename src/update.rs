use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde::Deserialize;
use tar::Archive;

const REPO: &str = "kyeotic/stack-sync";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn current_target() -> Result<&'static str> {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("aarch64-apple-darwin")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-apple-darwin")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("x86_64-unknown-linux-musl")
    } else {
        bail!("Unsupported platform for self-update")
    }
}

pub fn upgrade() -> Result<()> {
    if let Ok(exe) = std::env::current_exe() {
        if exe.to_string_lossy().contains("/nix/store/") {
            bail!(
                "This binary was installed via Nix. Update with:\n  \
                 nix profile upgrade --flake github:kyeotic/stack-sync"
            );
        }
    }

    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: v{}", current_version);

    let agent = ureq::Agent::new_with_defaults();
    let release: Release = agent
        .get(&format!(
            "https://api.github.com/repos/{}/releases/latest",
            REPO
        ))
        .header("User-Agent", "stack-sync")
        .call()?
        .body_mut()
        .read_json()
        .context("Failed to fetch latest release")?;

    let latest = release.tag_name.trim_start_matches('v');
    if latest == current_version {
        println!("Already up to date.");
        return Ok(());
    }

    println!("New version available: v{}", latest);

    let target = current_target()?;
    let asset_name = format!("stack-sync-{}.tar.gz", target);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .context(format!("No release asset found for {}", target))?;

    println!("Downloading {}...", asset.name);
    let response = agent
        .get(&asset.browser_download_url)
        .header("User-Agent", "stack-sync")
        .call()?;

    let decoder = GzDecoder::new(response.into_body().into_reader());
    let mut archive = Archive::new(decoder);

    let temp_dir = std::env::temp_dir().join("stack-sync-update");
    std::fs::create_dir_all(&temp_dir)?;

    archive.unpack(&temp_dir)?;

    let binary_path = temp_dir.join("stack-sync");
    if !binary_path.exists() {
        bail!("Binary not found in release archive");
    }

    self_replace::self_replace(&binary_path)?;
    std::fs::remove_dir_all(&temp_dir)?;

    println!("Updated to v{}", latest);
    Ok(())
}
