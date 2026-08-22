use crate::error::{Error, Result};
use crate::items::{self, catalog};
use crate::log;
use crate::store::Store;
use crate::types::{HardwareInfo, PendingRebootItem, RebootReviewItem, RebootState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Doc {
    boot_id: u64,
    items: Vec<PendingRebootItem>,
    review: Option<Vec<RebootReviewItem>>,
}

fn file(user: &Path) -> PathBuf {
    user.join("config").join("reboot-check.json")
}

pub fn boot_id() -> u64 {
    crate::backup::now_secs().saturating_sub(uptime_secs())
}

fn uptime_secs() -> u64 {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::SystemInformation::GetTickCount64() / 1000
    }
    #[cfg(not(windows))]
    {
        0
    }
}

fn same_boot(saved: u64, current: u64) -> bool {
    saved.abs_diff(current) <= 2
}

fn read_doc(user: &Path) -> Result<Option<Doc>> {
    let path = file(user);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
}

fn write_doc(user: &Path, doc: &Doc) -> Result<()> {
    fs::create_dir_all(user.join("config"))?;
    fs::write(file(user), serde_json::to_vec_pretty(doc)?)?;
    Ok(())
}

pub fn add_pending(user: &Path, items: &[PendingRebootItem]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let current = boot_id();
    let mut doc = match read_doc(user)? {
        Some(existing) if same_boot(existing.boot_id, current) => existing,
        _ => Doc {
            boot_id: current,
            items: Vec::new(),
            review: None,
        },
    };
    doc.review = None;
    doc.boot_id = current;
    for item in items {
        if !doc.items.iter().any(|i| i.id == item.id) {
            doc.items.push(item.clone());
        }
    }
    write_doc(user, &doc)
}

pub fn ack(user: &Path) -> Result<()> {
    let path = file(user);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn state(
    user: &Path,
    store: &dyn Store,
    hw: &HardwareInfo,
    game_path: Option<&str>,
) -> Result<RebootState> {
    state_at(user, store, hw, game_path, boot_id())
}

fn state_at(
    user: &Path,
    store: &dyn Store,
    hw: &HardwareInfo,
    game_path: Option<&str>,
    current_boot: u64,
) -> Result<RebootState> {
    let Some(mut doc) = read_doc(user)? else {
        return Ok(RebootState {
            needs_reboot: false,
            items: Vec::new(),
            review: None,
        });
    };
    if same_boot(doc.boot_id, current_boot) {
        return Ok(RebootState {
            needs_reboot: doc.review.is_none() && !doc.items.is_empty(),
            items: doc.items,
            review: doc.review,
        });
    }
    if doc.review.is_none() {
        let catalog = catalog(hw, game_path, None);
        let mut review = Vec::new();
        for pending in &doc.items {
            let Some(item) = catalog.iter().find(|i| i.id == pending.id) else {
                review.push(RebootReviewItem {
                    id: pending.id.clone(),
                    name: pending.name.clone(),
                    ok: false,
                    detail: "Unknown item.".into(),
                });
                continue;
            };
            let (ok, detail) = match items::item_state(item, store, hw) {
                Ok(true) => (true, "Matches the target.".to_string()),
                Ok(false) => (false, "Not at target after reboot.".to_string()),
                Err(e) => (false, e),
            };
            review.push(RebootReviewItem {
                id: pending.id.clone(),
                name: item.name.clone(),
                ok,
                detail,
            });
        }
        for row in &review {
            log::append(
                user,
                &format!(
                    "reboot-check {} {} {}",
                    row.id,
                    if row.ok { "ok" } else { "fail" },
                    row.detail
                ),
            )?;
        }
        doc.review = Some(review);
        write_doc(user, &doc)?;
    }
    Ok(RebootState {
        needs_reboot: false,
        items: doc.items,
        review: doc.review,
    })
}

fn is_dev_build() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_ascii_lowercase()))
        .is_some_and(|s| s.contains(r"\target\debug\"))
}

pub fn request() -> Result<()> {
    if is_dev_build() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let status = crate::win::hidden_command("shutdown")
            .args(["/r", "/t", "5", "/c", "Clean Optimizer"])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Msg("Windows declined the reboot request".into()))
        }
    }
    #[cfg(not(windows))]
    {
        Err(Error::Msg("Windows only".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use crate::types::HardwareInfo;
    use tempfile::tempdir;

    #[test]
    fn pending_stays_until_boot_changes() {
        let tmp = tempdir().unwrap();
        add_pending(
            tmp.path(),
            &[PendingRebootItem {
                id: "hags".into(),
                name: "Turn on Hardware-accelerated GPU Scheduling".into(),
            }],
        )
        .unwrap();
        let hw = HardwareInfo::fixture();
        let store = MemoryStore::new();
        let same = state_at(tmp.path(), &store, &hw, None, boot_id()).unwrap();
        assert!(same.needs_reboot);
        assert!(same.review.is_none());
        assert_eq!(same.items.len(), 1);
        let after = state_at(tmp.path(), &store, &hw, None, boot_id().saturating_add(1000)).unwrap();
        assert!(!after.needs_reboot);
        let review = after.review.expect("review");
        assert_eq!(review[0].id, "hags");
        assert!(!review[0].ok);
    }

    #[test]
    fn ack_clears_file() {
        let tmp = tempdir().unwrap();
        add_pending(
            tmp.path(),
            &[PendingRebootItem {
                id: "hags".into(),
                name: "Turn on Hardware-accelerated GPU Scheduling".into(),
            }],
        )
        .unwrap();
        ack(tmp.path()).unwrap();
        let hw = HardwareInfo::fixture();
        let store = MemoryStore::new();
        let empty = state_at(tmp.path(), &store, &hw, None, boot_id()).unwrap();
        assert!(!empty.needs_reboot);
        assert!(empty.review.is_none());
    }
}
