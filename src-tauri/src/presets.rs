use crate::error::{Error, Result};
use crate::types::Preset;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct SavedPreset {
    name: String,
    items: Vec<String>,
    saved: u64,
}

pub fn builtin() -> Vec<Preset> {
    vec![
        Preset {
            id: "main".into(),
            name: "Full set".into(),
            note: "Power, scheduling, IRQ pin, system trim, then GPU. Includes GPU name spoof on NVIDIA/AMD. Confirm that item separately. Mouse feel, sleep, search, and idle power change.".into(),
            builtin: true,
            items: vec![
                "power-ultimate","power-tuning","powerplan-lock",
                "prio-separation","game-priority","sys-responsiveness","mmcss-games","net-throttling-off","game-mode",
                "gpu-irq-affinity",
                "dvr-off","wer-off","sysmain-off","wsearch-off","hibernate-off",
                "paging-exec","transparency-off","mpo-off","dyntick-off","mouse-accel-off",
                "hags","fso-off","gpu-pref","gpu-pstate-lock","gpu-name-spoof",
                "pcie-check","vcredist-check","xmp-check",
            ].into_iter().map(str::to_string).collect(),
        },
        Preset {
            id: "balanced".into(),
            name: "Balanced".into(),
            note: "The 20 items with a clear gain and a small side effect. Leaves desktop look, mouse feel, services, and hibernate alone.".into(),
            builtin: true,
            items: vec![
                "power-ultimate","power-tuning","hags","game-mode","dvr-off","prio-separation",
                "paging-exec","wer-off","transparency-off","mpo-off","net-throttling-off",
                "sys-responsiveness","mmcss-games","fso-off","gpu-pref","game-priority",
                "pcie-check","vcredist-check","xmp-check",
            ].into_iter().map(str::to_string).collect(),
        },
        Preset {
            id: "safe-only".into(),
            name: "Current user only".into(),
            note: "HKCU only. Usually no reboot.".into(),
            builtin: true,
            items: vec![
                "game-mode","dvr-off","wer-off","transparency-off","fso-off","gpu-pref","windowed-opt-off",
            ].into_iter().map(str::to_string).collect(),
        },
    ]
}

pub fn load_all(root: &Path) -> Result<Vec<Preset>> {
    let mut out = builtin();
    let dir = root.join("profiles");
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw: SavedPreset = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom").to_string();
        out.push(Preset {
            id,
            name: raw.name,
            note: format!("Saved preset, {} items.", raw.items.len()),
            builtin: false,
            items: raw.items,
        });
    }
    Ok(out)
}

pub fn save(root: &Path, name: &str, items: Vec<String>) -> Result<Preset> {
    if name.trim().is_empty() {
        return Err(Error::Msg("preset name is empty".into()));
    }
    if builtin().iter().any(|p| p.id.eq_ignore_ascii_case(name) || p.name.eq_ignore_ascii_case(name)) {
        return Err(Error::Msg("cannot overwrite a built-in preset".into()));
    }
    let dir = root.join("profiles");
    fs::create_dir_all(&dir)?;
    let id = sanitize_id(name);
    let saved = SavedPreset {
        name: name.to_string(),
        items: items.clone(),
        saved: crate::backup::now_secs(),
    };
    fs::write(dir.join(format!("{id}.json")), serde_json::to_vec_pretty(&saved)?)?;
    Ok(Preset {
        id,
        name: name.to_string(),
        note: format!("Saved preset, {} items.", items.len()),
        builtin: false,
        items,
    })
}

pub fn delete(root: &Path, id: &str) -> Result<()> {
    if builtin().iter().any(|p| p.id == id) {
        return Err(Error::Msg("built-in presets cannot be deleted".into()));
    }
    let path = root.join("profiles").join(format!("{id}.json"));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn sanitize_id(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "preset".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_order_starts_with_main() {
        assert_eq!(builtin()[0].id, "main");
        assert!(builtin()[0].items.contains(&"gpu-name-spoof".into()));
        assert!(!builtin()[1].items.contains(&"gpu-name-spoof".into()));
    }

    #[test]
    fn save_and_delete_user_preset() {
        let tmp = tempdir().unwrap();
        save(tmp.path(), "Night", vec!["dvr-off".into()]).unwrap();
        let all = load_all(tmp.path()).unwrap();
        assert!(all.iter().any(|p| p.name == "Night"));
        delete(tmp.path(), "Night").unwrap();
        let all = load_all(tmp.path()).unwrap();
        assert!(!all.iter().any(|p| p.name == "Night"));
    }
}
