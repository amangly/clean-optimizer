use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

const REPO: &str = "amangly/clean-optimizer";
const LATEST_API: &str = "https://api.github.com/repos/amangly/clean-optimizer/releases/latest";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub notes: String,
    pub setup_url: Option<String>,
    pub available: bool,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
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
        Err(ureq::Error::Status(404, _)) => Ok(UpdateInfo {
            current: current.to_string(),
            latest: None,
            notes: "No GitHub release yet.".into(),
            setup_url: None,
            available: false,
        }),
        Err(_) => Ok(UpdateInfo {
            current: current.to_string(),
            latest: None,
            notes: "Could not reach GitHub.".into(),
            setup_url: None,
            available: false,
        }),
    }
}

pub fn parse_release(current: &str, json: &str) -> Result<UpdateInfo> {
    let rel: GhRelease = serde_json::from_str(json)?;
    validate_setup_url(&rel.html_url)?;
    let latest = rel.tag_name.trim().trim_start_matches('v').to_string();
    let available = is_newer(&latest, current);
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: Some(latest),
        notes: rel.body.unwrap_or_default(),
        setup_url: Some(rel.html_url),
        available,
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
    raw
        .trim()
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
        assert!(validate_setup_url("https://github.com/amangly/clean-optimizer/releases/tag/v0.2.0").is_ok());
    }

    #[test]
    fn parse_marks_newer_tag() {
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/amangly/clean-optimizer/releases/tag/v0.2.0","body":"fix"}"#;
        let info = parse_release("0.1.0", json).unwrap();
        assert!(info.available);
        assert_eq!(info.latest.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn parse_same_version_is_current() {
        let json = r#"{"tag_name":"v0.1.0","html_url":"https://github.com/amangly/clean-optimizer/releases/tag/v0.1.0","body":""}"#;
        let info = parse_release("0.1.0", json).unwrap();
        assert!(!info.available);
    }
}
