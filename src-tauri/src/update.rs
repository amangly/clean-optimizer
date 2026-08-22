use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const REPO: &str = "amangly/clean-optimizer";
const LATEST_API: &str = "https://api.github.com/repos/amangly/clean-optimizer/releases/latest";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub notes: String,
    pub setup_url: Option<String>,
    pub asset_url: Option<String>,
    pub sha256: Option<String>,
    pub available: bool,
    pub reached: bool,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

pub fn check(current: &str) -> Result<UpdateInfo> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(8))
        .build();
    match agent
        .get(LATEST_API)
        .set("User-Agent", "CleanOptimizer")
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            parse_release(current, &text)
        }
        Err(ureq::Error::Status(404, _)) => Ok(empty(current, "No GitHub release yet.")),
        Err(_) => Ok(empty(current, "Could not reach GitHub.")),
    }
}

fn empty(current: &str, notes: &str) -> UpdateInfo {
    UpdateInfo {
        current: current.to_string(),
        latest: None,
        notes: notes.into(),
        setup_url: None,
        asset_url: None,
        sha256: None,
        available: false,
        reached: false,
    }
}

pub fn parse_release(current: &str, json: &str) -> Result<UpdateInfo> {
    let rel: GhRelease = serde_json::from_str(json)?;
    validate_setup_url(&rel.html_url)?;
    let latest = rel.tag_name.trim().trim_start_matches('v').to_string();
    let available = is_newer(&latest, current);
    let asset = rel
        .assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(".exe"));
    if let Some(asset) = asset {
        validate_setup_url(&asset.browser_download_url)?;
    }
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: Some(latest),
        notes: rel.body.unwrap_or_default(),
        setup_url: Some(rel.html_url),
        asset_url: asset.map(|a| a.browser_download_url.clone()),
        sha256: asset.and_then(|a| a.digest.as_deref()).and_then(strip_sha),
        available,
        reached: true,
    })
}

fn strip_sha(raw: &str) -> Option<String> {
    let hex = raw.strip_prefix("sha256:").unwrap_or(raw).trim();
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

pub fn apply_latest(current: &str) -> Result<PathBuf> {
    let info = check(current)?;
    if !info.available {
        return Err(Error::Msg("already on the latest GitHub release".into()));
    }
    let dest = std::env::temp_dir().join("clean-optimizer-update");
    let path = download(&info, &dest)?;
    start_installer(&path)
}

fn start_installer(path: &Path) -> Result<PathBuf> {
    std::process::Command::new(path)
        .spawn()
        .map_err(|e| Error::Msg(e.to_string()))?;
    std::process::exit(0);
}

pub fn download(info: &UpdateInfo, dest_dir: &Path) -> Result<PathBuf> {
    let url = info
        .asset_url
        .as_ref()
        .ok_or_else(|| Error::Msg("no installer asset on the latest release".into()))?;
    validate_setup_url(url)?;
    let name = url.rsplit('/').next().unwrap_or("setup.exe");
    fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join(name);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "CleanOptimizer")
        .call()
        .map_err(|e| Error::Msg(e.to_string()))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| Error::Msg(e.to_string()))?;
    if let Some(expect) = &info.sha256 {
        let got = sha256_hex(&bytes);
        if got != *expect {
            return Err(Error::Msg("installer SHA256 does not match the release".into()));
        }
    }
    fs::write(&dest, bytes)?;
    Ok(dest)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

pub fn host_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    Some(rest.split('/').next()?.to_ascii_lowercase())
}

pub fn validate_setup_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(Error::Msg("setup URL must be https".into()));
    }
    let host = host_of(url).ok_or_else(|| Error::Msg("setup URL host missing".into()))?;
    if host != "github.com" && host != "api.github.com" {
        return Err(Error::Msg(format!("setup host {host} is not allowed")));
    }
    let path = url.splitn(4, '/').nth(3).unwrap_or("");
    let expected = if host == "api.github.com" {
        format!("repos/{REPO}/")
    } else {
        format!("{REPO}/")
    };
    if !path.to_ascii_lowercase().starts_with(&expected) {
        return Err(Error::Msg("setup URL is not this repo".into()));
    }
    Ok(())
}

fn is_newer(latest: &str, current: &str) -> bool {
    ver_parts(latest) > ver_parts(current)
}

fn ver_parts(raw: &str) -> Vec<u32> {
    raw.trim()
        .trim_start_matches('v')
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_foreign_host() {
        assert!(validate_setup_url("https://evil.example/setup.exe").is_err());
        assert!(validate_setup_url("https://github.com/other/tool/releases/tag/v1").is_err());
        assert!(
            validate_setup_url("https://github.com/amangly/clean-optimizer/releases/tag/v0.2.0")
                .is_ok()
        );
    }

    #[test]
    fn parse_marks_newer_tag() {
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/amangly/clean-optimizer/releases/tag/v0.2.0","body":"fix"}"#;
        let info = parse_release("0.1.0", json).unwrap();
        assert!(info.available);
        assert!(info.reached);
        assert_eq!(info.latest.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn parse_same_version_is_current() {
        let json = r#"{"tag_name":"v0.1.0","html_url":"https://github.com/amangly/clean-optimizer/releases/tag/v0.1.0","body":""}"#;
        let info = parse_release("0.1.0", json).unwrap();
        assert!(!info.available);
    }

    #[test]
    fn parse_picks_exe_asset_and_digest() {
        let json = r#"{
            "tag_name":"v0.2.0",
            "html_url":"https://github.com/amangly/clean-optimizer/releases/tag/v0.2.0",
            "body":"",
            "assets":[{
                "name":"Clean.Optimizer_0.2.0_x64-setup.exe",
                "browser_download_url":"https://github.com/amangly/clean-optimizer/releases/download/v0.2.0/Clean.Optimizer_0.2.0_x64-setup.exe",
                "digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }]
        }"#;
        let info = parse_release("0.1.0", json).unwrap();
        assert!(info.asset_url.unwrap().ends_with(".exe"));
        assert_eq!(
            info.sha256.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }
}
