use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheReport {
    pub deleted_files: u32,
    pub bytes: u64,
    pub skipped: u32,
    pub paths: Vec<String>,
}

pub fn shader_dirs() -> Vec<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    if local.is_empty() {
        return Vec::new();
    }
    let local = PathBuf::from(local);
    vec![
        local.join(r"NVIDIA\DXCache"),
        local.join(r"NVIDIA\GLCache"),
        local.join(r"NVIDIA Corporation\NV_Cache"),
        local.join(r"AMD\DxCache"),
        local.join(r"AMD\GLCache"),
        local.join(r"D3DSCache"),
        local.join(r"Microsoft\D3DSCache"),
    ]
}

pub fn clean(dirs: &[PathBuf]) -> Result<CacheReport> {
    let mut deleted_files = 0u32;
    let mut bytes = 0u64;
    let mut skipped = 0u32;
    let mut paths = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        paths.push(dir.display().to_string());
        for entry in walk(dir) {
            match fs::metadata(&entry) {
                Ok(meta) => {
                    let size = meta.len();
                    match fs::remove_file(&entry) {
                        Ok(()) => {
                            deleted_files += 1;
                            bytes += size;
                        }
                        Err(_) => skipped += 1,
                    }
                }
                Err(_) => skipped += 1,
            }
        }
    }
    Ok(CacheReport {
        deleted_files,
        bytes,
        skipped,
        paths,
    })
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn deletes_files_under_cache_dir() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("DXCache");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.bin"), vec![0u8; 32]).unwrap();
        let report = clean(&[dir]).unwrap();
        assert_eq!(report.deleted_files, 1);
        assert_eq!(report.bytes, 32);
    }
}
