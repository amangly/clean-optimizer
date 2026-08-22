use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub const LIBRARY_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub group_id: String,
    pub variant_id: String,
    pub display_name: String,
    pub item_ids: Vec<String>,
    pub item_set_hash: String,
    pub purpose: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentState {
    pub schema_version: u32,
    pub library_version: u32,
    pub experiment_id: String,
    pub status: String,
    pub scene_id: String,
    pub current_group: Option<String>,
    pub baseline_runs: u32,
    pub kept: Vec<String>,
    pub rolled_back: Vec<String>,
}

pub fn library() -> Vec<Candidate> {
    vec![
        Candidate {
            group_id: "G1".into(),
            variant_id: "background_low_risk".into(),
            display_name: "Background and Game Mode".into(),
            item_ids: vec!["game-mode".into(), "dvr-off".into()],
            item_set_hash: item_set_hash(&["game-mode".into(), "dvr-off".into()]),
            purpose: "Cut background capture and raise Game Mode. Watch 1% lows and hitches.".into(),
        },
        Candidate {
            group_id: "G2".into(),
            variant_id: "foreground_scheduler".into(),
            display_name: "Foreground scheduling".into(),
            item_ids: vec![
                "prio-separation".into(),
                "game-priority".into(),
                "mmcss-games".into(),
                "net-throttling-off".into(),
            ],
            item_set_hash: item_set_hash(&[
                "prio-separation".into(),
                "game-priority".into(),
                "mmcss-games".into(),
                "net-throttling-off".into(),
            ]),
            purpose: "Raise CPU, IO, and multimedia priority. Watch smoothness.".into(),
        },
        Candidate {
            group_id: "G3".into(),
            variant_id: "display_path".into(),
            display_name: "Display and GPU pick".into(),
            item_ids: vec!["fso-off".into(), "gpu-pref".into(), "windowed-opt-off".into()],
            item_set_hash: item_set_hash(&["fso-off".into(), "gpu-pref".into(), "windowed-opt-off".into()]),
            purpose: "Fix mixed-GPU pick and the present path. Restart the game before this round.".into(),
        },
    ]
}

pub fn item_set_hash(ids: &[String]) -> String {
    let mut clean: Vec<String> = ids.iter().map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect();
    clean.sort();
    clean.dedup();
    sha256_hex(&clean.join(","))
}

pub fn start(root: &Path, scene_id: &str) -> Result<ExperimentState> {
    let scene = scene_id.trim();
    if scene.len() < 2 || scene.len() > 80 {
        return Err(Error::Msg("scene id must be 2-80 characters".into()));
    }
    let state = ExperimentState {
        schema_version: 1,
        library_version: LIBRARY_VERSION,
        experiment_id: format!("exp_{}", uuid::Uuid::new_v4().as_simple()),
        status: "baseline_pending".into(),
        scene_id: scene.to_string(),
        current_group: None,
        baseline_runs: 0,
        kept: vec![],
        rolled_back: vec![],
    };
    write_state(root, &state)?;
    Ok(state)
}

pub fn load(root: &Path) -> Result<Option<ExperimentState>> {
    let path = state_path(root);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
}

pub fn confirm_round(root: &Path, avg_fps: f64, low_1pct: f64, hitches: u32) -> Result<ExperimentState> {
    let mut state = load(root)?.ok_or_else(|| Error::Msg("no experiment in progress".into()))?;
    match state.status.as_str() {
        "baseline_pending" | "baseline_running" => {
            state.baseline_runs += 1;
            state.status = if state.baseline_runs >= 3 {
                "variant_pending".into()
            } else {
                "baseline_pending".into()
            };
            if state.status == "variant_pending" {
                state.current_group = Some("G1".into());
            }
        }
        "variant_pending" | "variant_running" | "variant_complete" => {
            let keep = avg_fps > 0.0 && low_1pct + 0.5 >= 0.0 && hitches < 30;
            let group = state.current_group.clone().unwrap_or_else(|| "G1".into());
            if keep {
                state.kept.push(group.clone());
            } else {
                state.rolled_back.push(group.clone());
            }
            state.current_group = next_group(&group);
            state.status = if state.current_group.is_none() {
                "completed".into()
            } else {
                "variant_pending".into()
            };
        }
        _ => return Err(Error::Msg(format!("cannot confirm from {}", state.status))),
    }
    write_state(root, &state)?;
    Ok(state)
}

pub fn cancel(root: &Path) -> Result<ExperimentState> {
    let mut state = load(root)?.ok_or_else(|| Error::Msg("no experiment in progress".into()))?;
    state.status = "cancelled".into();
    write_state(root, &state)?;
    Ok(state)
}

fn next_group(current: &str) -> Option<String> {
    match current {
        "G1" => Some("G2".into()),
        "G2" => Some("G3".into()),
        _ => None,
    }
}

fn state_path(root: &Path) -> std::path::PathBuf {
    root.join("config").join("experiment.json")
}

fn write_state(root: &Path, state: &ExperimentState) -> Result<()> {
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn library_is_low_risk_only() {
        let ids: Vec<_> = library().into_iter().flat_map(|c| c.item_ids).collect();
        assert!(!ids.iter().any(|i| i == "gpu-name-spoof"));
        assert_eq!(library().len(), 3);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(
            item_set_hash(&["dvr-off".into(), "game-mode".into()]),
            item_set_hash(&["GAME-MODE".into(), "dvr-off".into()])
        );
    }

    #[test]
    fn experiment_reaches_complete() {
        let tmp = tempdir().unwrap();
        start(tmp.path(), "farm-east").unwrap();
        let mut state = None;
        for _ in 0..3 {
            state = Some(confirm_round(tmp.path(), 120.0, 80.0, 2).unwrap());
        }
        for _ in 0..3 {
            state = Some(confirm_round(tmp.path(), 130.0, 90.0, 1).unwrap());
        }
        assert_eq!(state.unwrap().status, "completed");
    }
}
