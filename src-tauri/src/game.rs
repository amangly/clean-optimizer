use crate::apply::valid_game_path;
use crate::error::Result;
use crate::items::GAME_EXES;
use std::path::{Path, PathBuf};

pub fn find_game() -> Result<Option<String>> {
    if let Some(p) = from_process() {
        return Ok(Some(p));
    }
    let mut roots = Vec::new();
    roots.extend(from_uninstall());
    roots.extend(from_platforms());
    roots.extend(from_drive_guesses());
    for root in roots {
        if let Some(found) = search_root(&root) {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

pub fn pick_game() -> Result<Option<String>> {
    let picked = rfd::FileDialog::new()
        .add_filter("Executable", &["exe"])
        .set_title("Delta Force exe (optional)")
        .pick_file();
    match picked {
        Some(path) => {
            let text = path.to_string_lossy().to_string();
            if valid_game_path(&text) {
                Ok(Some(text))
            } else {
                Err(crate::error::Error::BadGamePath)
            }
        }
        None => Ok(None),
    }
}

fn from_process() -> Option<String> {
    #[cfg(windows)]
    {
        for name in ["DeltaForceClient-Win64-Shipping", "DeltaForceClient", "DeltaForce"] {
            if let Some(path) = crate::win::process_path(name) {
                if path.contains("Shipping") || valid_game_path(&path) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn from_uninstall() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        crate::win::uninstall_roots()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

fn from_platforms() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        crate::win::platform_roots()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

fn from_drive_guesses() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if !root.exists() {
            continue;
        }
        for guess in ["Delta Force", "WeGame", "WeGameApps", r"Program Files\WeGame"] {
            let p = root.join(guess);
            if p.exists() {
                out.push(p);
            }
        }
    }
    out
}

fn search_root(root: &Path) -> Option<String> {
    for rel in [
        r"DeltaForce\Binaries\Win64\DeltaForceClient-Win64-Shipping.exe",
        r"Delta Force\DeltaForce\Binaries\Win64\DeltaForceClient-Win64-Shipping.exe",
        r"Binaries\Win64\DeltaForceClient-Win64-Shipping.exe",
    ] {
        let p = root.join(rel);
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    walk_for_exe(root, 6)
}

fn walk_for_exe(root: &Path, depth: u32) -> Option<String> {
    if depth == 0 || !root.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if GAME_EXES.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    for dir in dirs {
        if let Some(found) = walk_for_exe(&dir, depth - 1) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn finds_shipping_layout() {
        let tmp = tempdir().unwrap();
        let exe = tmp
            .path()
            .join(r"DeltaForce\Binaries\Win64\DeltaForceClient-Win64-Shipping.exe");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"mz").unwrap();
        assert_eq!(search_root(tmp.path()).as_deref(), Some(exe.to_str().unwrap()));
    }
}
