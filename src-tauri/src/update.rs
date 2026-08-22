use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

const ALLOWED_HOST: &str = "updates.cleanoptimizer.app";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub notes: String,
    pub setup_url: Option<String>,
    pub available: bool,
}

pub fn check(current: &str, manifest_url: Option<&str>) -> Result<UpdateInfo> {
    if let Some(url) = manifest_url {
        validate_setup_url(url)?;
    }
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: None,
        notes: "Update check is local-only until you point this build at an HTTPS manifest.".into(),
        setup_url: None,
        available: false,
    })
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
    if host != ALLOWED_HOST {
        return Err(Error::Msg(format!("setup host {host} is not allowed")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_foreign_host() {
        assert!(validate_setup_url("https://evil.example/setup.exe").is_err());
        assert!(validate_setup_url("https://updates.cleanoptimizer.app/CleanOptimizer-Setup.exe").is_ok());
    }
}
