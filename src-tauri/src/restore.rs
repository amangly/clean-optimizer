use crate::backup::{BackupDoc, BackupOp, BackupStore};
use crate::error::Result;
use crate::items::SELECTIVE_RESTORE;
use crate::store::{RegKeyRef, Store};
use crate::types::{Hive, ItemResult, RegVal, RestoreItem, RestoreReport};

pub fn list_restore(store: &dyn Store, backups: &BackupStore) -> Result<Vec<RestoreItem>> {
    let merged = merge_originals(&backups.list_active()?);
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for op in merged {
        if !seen.insert(op.item_id.clone()) {
            continue;
        }
        let selective = SELECTIVE_RESTORE.contains(&op.item_id.as_str());
        let conflict = is_conflict(store, &op)?;
        out.push(RestoreItem {
            id: op.item_id.clone(),
            name: crate::items::display_name(&op.item_id),
            selective,
            conflict,
            detail: if conflict {
                "Current value is no longer what this app wrote.".into()
            } else {
                "Can restore the first original value.".into()
            },
        });
    }
    Ok(out)
}

pub fn restore(
    store: &dyn Store,
    backups: &BackupStore,
    items: Option<Vec<String>>,
) -> Result<RestoreReport> {
    let docs = backups.list_active()?;
    let merged = merge_originals(&docs);
    let filter = items.map(|v| v.into_iter().collect::<std::collections::HashSet<_>>());
    let mut results = Vec::new();
    let mut restored = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let mut notes = Vec::new();

    for op in merged {
        if let Some(filter) = &filter {
            if !filter.contains(&op.item_id) {
                continue;
            }
            if !SELECTIVE_RESTORE.contains(&op.item_id.as_str()) {
                results.push(ItemResult {
                    id: op.item_id.clone(),
                    name: op.item_id.clone(),
                    ok: false,
                    changed: false,
                    skipped: false,
                    attention: false,
                    reboot: false,
                    message: "Selective restore is not open for this item. Use full restore.".into(),
                });
                failed += 1;
                continue;
            }
        }
        if is_conflict(store, &op)? {
            results.push(ItemResult {
                id: op.item_id.clone(),
                name: op.item_id.clone(),
                ok: true,
                changed: false,
                skipped: true,
                attention: false,
                reboot: false,
                message: "Left in place. Something else changed this setting after apply.".into(),
            });
            skipped += 1;
            continue;
        }
        match restore_op(store, &op) {
            Ok(true) => {
                restored += 1;
                results.push(ItemResult {
                    id: op.item_id.clone(),
                    name: op.item_id.clone(),
                    ok: true,
                    changed: true,
                    skipped: false,
                    attention: false,
                    reboot: false,
                    message: "Restored the original value.".into(),
                });
            }
            Ok(false) => {
                skipped += 1;
                results.push(ItemResult {
                    id: op.item_id.clone(),
                    name: op.item_id.clone(),
                    ok: true,
                    changed: false,
                    skipped: true,
                    attention: false,
                    reboot: false,
                    message: "Already at the original value.".into(),
                });
            }
            Err(e) => {
                failed += 1;
                results.push(ItemResult {
                    id: op.item_id.clone(),
                    name: op.item_id.clone(),
                    ok: false,
                    changed: false,
                    skipped: false,
                    attention: false,
                    reboot: false,
                    message: e.to_string(),
                });
            }
        }
    }

    if restored > 0 {
        if let Some(first) = docs.first() {
            let ids: Vec<String> = results.iter().filter(|r| r.changed).map(|r| r.id.clone()).collect();
            backups.write_receipt(&first.apply_id, &ids)?;
        }
        notes.push("Original v3 backups stay on disk. A restore receipt marks the consumed ops.".into());
    }

    Ok(RestoreReport {
        restored,
        failed,
        skipped,
        notes,
        results,
    })
}

pub fn merge_originals(docs: &[BackupDoc]) -> Vec<BackupOp> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for doc in docs {
        for op in &doc.ops {
            let key = (op.kind.clone(), op.target.clone());
            if seen.insert(key) {
                out.push(op.clone());
            }
        }
    }
    out
}

fn is_conflict(store: &dyn Store, op: &BackupOp) -> Result<bool> {
    match op.kind.as_str() {
        "reg" => {
            if let Some(written) = serde_json::from_value::<Option<RegVal>>(op.written.clone()).ok().flatten() {
                if let Some(key) = parse_reg_target(&op.target) {
                    return Ok(store.get_reg(&key)? != Some(written));
                }
            }
            Ok(false)
        }
        "power" => {
            let written = op.written.as_str().unwrap_or_default();
            Ok(store.active_scheme()?.as_deref() != Some(written))
        }
        "sched" => Ok(store.lock_task()? != op.written.as_bool().unwrap_or(true)),
        _ => Ok(false),
    }
}

fn restore_op(store: &dyn Store, op: &BackupOp) -> Result<bool> {
    match op.kind.as_str() {
        "reg" | "kvstr" => {
            let Some(key) = parse_reg_target(&op.target) else {
                if op.kind == "kvstr" {
                    return Ok(false);
                }
                return Ok(false);
            };
            match serde_json::from_value::<Option<RegVal>>(op.original.clone())? {
                Some(val) => {
                    store.set_reg(&key, &val)?;
                    Ok(true)
                }
                None => {
                    store.delete_reg(&key)?;
                    Ok(true)
                }
            }
        }
        "power" => {
            match op.original.as_str() {
                Some(guid) => store.set_active_scheme(guid)?,
                None => store.set_active_scheme(crate::store::BALANCED_SCHEME)?,
            }
            Ok(true)
        }
        "pcfg" => {
            if let Some(v) = op.original.as_u64() {
                let mut parts = op.target.split('\\');
                let sub = parts.next().unwrap_or("");
                let setting = parts.next().unwrap_or("");
                store.set_power_setting(sub, setting, v as u32)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        "mmagent" => {
            if let Some(on) = op.original.as_bool() {
                store.set_mmagent(&op.target, on)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        "hib" => {
            store.set_hibernate(op.original.as_bool().unwrap_or(true))?;
            Ok(true)
        }
        "bcd" => {
            if let Some(v) = op.original.as_str() {
                store.set_bcd(&op.target, v)?;
            } else {
                store.delete_bcd(&op.target)?;
            }
            Ok(true)
        }
        "sched" => {
            store.set_lock_task(op.original.as_bool().unwrap_or(false))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_reg_target(target: &str) -> Option<RegKeyRef> {
    let rest = target.strip_prefix("Hkcu\\").or_else(|| target.strip_prefix("Hklm\\"))?;
    let hive = if target.starts_with("Hkcu") {
        Hive::Hkcu
    } else {
        Hive::Hklm
    };
    let (path, name) = rest.rsplit_once('\\')?;
    Some(RegKeyRef::new(hive, path, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::{apply, ApplyRequest};
    use crate::store::MemoryStore;
    use crate::types::HardwareInfo;
    use tempfile::tempdir;

    #[test]
    fn restore_returns_first_original() {
        let tmp = tempdir().unwrap();
        let backups = BackupStore::open(tmp.path()).unwrap();
        let store = MemoryStore::new();
        let hw = HardwareInfo::fixture();
        let req = ApplyRequest {
            items: vec!["game-mode".into()],
            preset: None,
            game_path: None,
            gpu_spoof_model: None,
            risky: false,
            admin: true,
        };
        apply(&store, &backups, &hw, req, None).unwrap();
        let report = restore(&store, &backups, Some(vec!["game-mode".into()])).unwrap();
        assert_eq!(report.failed, 0);
        assert!(report.restored >= 1);
        let listed = list_restore(&store, &backups).unwrap();
        assert!(listed.iter().any(|i| i.id == "game-mode"));
    }

    #[test]
    fn earliest_original_wins() {
        let older = BackupOp {
            item_id: "game-mode".into(),
            kind: "reg".into(),
            target: r"Hkcu\Software\Microsoft\GameBar\AutoGameModeEnabled".into(),
            original: serde_json::json!(null),
            written: serde_json::json!({"kind":"dword","value":1}),
        };
        let newer = BackupOp {
            item_id: "game-mode".into(),
            kind: "reg".into(),
            target: r"Hkcu\Software\Microsoft\GameBar\AutoGameModeEnabled".into(),
            original: serde_json::json!({"kind":"dword","value":1}),
            written: serde_json::json!({"kind":"dword","value":1}),
        };
        let docs = vec![
            BackupDoc {
                schema: 3,
                apply_id: "a".into(),
                created: 1,
                items: vec!["game-mode".into()],
                ops: vec![older.clone()],
                hmac: String::new(),
            },
            BackupDoc {
                schema: 3,
                apply_id: "b".into(),
                created: 2,
                items: vec!["game-mode".into()],
                ops: vec![newer],
                hmac: String::new(),
            },
        ];
        let merged = merge_originals(&docs);
        assert_eq!(merged[0].original, serde_json::Value::Null);
    }
}
