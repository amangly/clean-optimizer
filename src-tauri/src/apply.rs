use crate::backup::{BackupOp, BackupStore};
use crate::error::{Error, Result};
use crate::items::{catalog, item_state, Op, OptItem, GAME_EXES};
use crate::store::{set_kv_item, RegKeyRef, Store};
use crate::types::{ApplyReport, HardwareInfo, ItemResult, RegVal, Tier};

pub struct ApplyRequest {
    pub items: Vec<String>,
    pub preset: Option<String>,
    pub game_path: Option<String>,
    pub gpu_spoof_model: Option<String>,
    pub risky: bool,
    pub admin: bool,
}

pub fn resolve_ids(req: &ApplyRequest, preset_items: Option<&[String]>, catalog_items: &[OptItem]) -> Vec<String> {
    if let Some(preset) = preset_items {
        return preset.to_vec();
    }
    if !req.items.is_empty() {
        return req.items.clone();
    }
    catalog_items
        .iter()
        .filter(|i| i.default && i.bulk_select && i.tier == Tier::Safe)
        .map(|i| i.id.clone())
        .collect()
}

pub fn apply(
    store: &dyn Store,
    backups: &BackupStore,
    hw: &HardwareInfo,
    req: ApplyRequest,
    preset_items: Option<Vec<String>>,
) -> Result<ApplyReport> {
    if let Some(name) = &req.preset {
        if preset_items.is_none() {
            return Err(Error::Msg(format!("unknown preset {name}")));
        }
    }
    let all = catalog(hw, req.game_path.as_deref(), req.gpu_spoof_model.as_deref());
    let ids = resolve_ids(&req, preset_items.as_deref(), &all);
    let mut results = Vec::new();
    let mut ops = Vec::new();
    let mut applied_ids = Vec::new();

    for id in &ids {
        let Some(item) = all.iter().find(|i| i.id == *id) else {
            results.push(fail(id, id, format!("unknown item {id}")));
            continue;
        };
        if item.tier == Tier::Risky && !req.risky {
            results.push(fail(&item.id, &item.name, Error::Risky(item.id.clone()).to_string()));
            continue;
        }
        if item.admin && !req.admin {
            results.push(fail(&item.id, &item.name, Error::AdminRequired(item.id.clone()).to_string()));
            continue;
        }
        if item.requires_game && req.game_path.is_none() {
            results.push(skip(&item.id, &item.name, "No game path.".into(), false));
            continue;
        }
        if item.requires_game {
            if let Some(path) = &req.game_path {
                if !valid_game_path(path) {
                    results.push(fail(&item.id, &item.name, Error::BadGamePath.to_string()));
                    continue;
                }
            }
        }
            match apply_item(store, hw, item) {
            Ok(outcome) => {
                if outcome.changed {
                    ops.extend(outcome.ops);
                    applied_ids.push(item.id.clone());
                }
                results.push(ItemResult {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    ok: true,
                    changed: outcome.changed,
                    skipped: outcome.skipped,
                    attention: outcome.attention,
                    reboot: item.reboot && outcome.changed,
                    message: outcome.message,
                });
            }
            Err(e) => results.push(fail(&item.id, &item.name, e.to_string())),
        }
    }

    let backup_file = if ops.is_empty() {
        None
    } else {
        let (_, path) = backups.write(&applied_ids, ops)?;
        Some(path.display().to_string())
    };

    let succeeded = results.iter().filter(|r| r.ok && !r.skipped).count() as u32;
    let failed = results.iter().filter(|r| !r.ok).count() as u32;
    let skipped = results.iter().filter(|r| r.skipped).count() as u32;
    let attention = results.iter().filter(|r| r.attention).count() as u32;

    Ok(ApplyReport {
        apply_id: backup_file
            .as_ref()
            .and_then(|p| PathFileStem::stem(p))
            .unwrap_or_default(),
        results,
        backup_file,
        succeeded,
        failed,
        skipped,
        attention,
    })
}

struct PathFileStem;

impl PathFileStem {
    fn stem(path: &str) -> Option<String> {
        std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.trim_start_matches("backup-").to_string())
    }
}

struct Outcome {
    changed: bool,
    skipped: bool,
    attention: bool,
    message: String,
    ops: Vec<BackupOp>,
}

fn apply_item(store: &dyn Store, hw: &HardwareInfo, item: &OptItem) -> Result<Outcome> {
    match item.kind {
        crate::types::ItemKind::Check => {
            let check = crate::checks::run_all()
                .into_iter()
                .find(|c| c.id == item.id);
            Ok(Outcome {
                changed: false,
                skipped: true,
                attention: check.as_ref().map(|c| c.attention).unwrap_or(false),
                message: check
                    .map(|c| c.text)
                    .unwrap_or_else(|| "Check only.".into()),
                ops: vec![],
            })
        }
        crate::types::ItemKind::Cache => {
            let report = crate::cache::clean(&crate::cache::shader_dirs())?;
            Ok(Outcome {
                changed: report.deleted_files > 0,
                skipped: report.deleted_files == 0,
                attention: false,
                message: format!(
                    "Deleted {} files ({} bytes). Skipped {}.",
                    report.deleted_files, report.bytes, report.skipped
                ),
                ops: vec![],
            })
        }
        _ => {
            if item_state(item, store, hw).unwrap_or(false) {
                return Ok(Outcome {
                    changed: false,
                    skipped: true,
                    attention: false,
                    message: "Already at the target.".into(),
                    ops: vec![],
                });
            }
            let mut recorded = Vec::new();
            for op in &item.ops {
                if let Some(row) = apply_op(store, hw, item, op)? {
                    recorded.push(row);
                }
            }
            Ok(Outcome {
                changed: !recorded.is_empty(),
                skipped: recorded.is_empty(),
                attention: false,
                message: if recorded.is_empty() {
                    "Nothing to write.".into()
                } else {
                    format!("Wrote {} change(s).", recorded.len())
                },
                ops: recorded,
            })
        }
    }
}

fn apply_op(
    store: &dyn Store,
    hw: &HardwareInfo,
    item: &OptItem,
    op: &Op,
) -> Result<Option<BackupOp>> {
    match op {
        Op::Reg { hive, path, name, value } => {
            let key = RegKeyRef::new(*hive, path, name);
            let original = store.get_reg(&key)?;
            if original.as_ref() == Some(value) {
                return Ok(None);
            }
            store.set_reg(&key, value)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "reg".into(),
                target: format!("{hive:?}\\{path}\\{name}"),
                original: serde_json::to_value(&original)?,
                written: serde_json::to_value(value)?,
            }))
        }
        Op::PowerUltimate => {
            let original = store.active_scheme()?;
            let tool = store.ensure_tool_scheme()?;
            if original
                .as_ref()
                .is_some_and(|got| crate::store::guid_eq(got, &tool))
            {
                return Ok(None);
            }
            store.set_active_scheme(&tool)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "power".into(),
                target: "active-scheme".into(),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(tool),
            }))
        }
        Op::PowerCfg { sub, setting, value, optional } => {
            let original = store.power_setting(sub, setting)?;
            if original == Some(*value) {
                return Ok(None);
            }
            if original.is_none() && *optional {
                return Ok(None);
            }
            store.set_power_setting(sub, setting, *value)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "pcfg".into(),
                target: format!("{sub}\\{setting}"),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(value),
            }))
        }
        Op::MmAgent { feature, enabled } => {
            let original = store.mmagent(feature)?;
            if original == Some(*enabled) {
                return Ok(None);
            }
            store.set_mmagent(feature, *enabled)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "mmagent".into(),
                target: feature.clone(),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(enabled),
            }))
        }
        Op::HibernateOff => {
            let original = store.hibernate()?;
            if original == Some(false) {
                return Ok(None);
            }
            store.set_hibernate(false)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "hib".into(),
                target: "hibernate".into(),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(false),
            }))
        }
        Op::Bcd { name, value } => {
            let original = store.bcd(name)?;
            if original
                .as_deref()
                .is_some_and(|got| got.eq_ignore_ascii_case(value))
            {
                return Ok(None);
            }
            store.set_bcd(name, value)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "bcd".into(),
                target: name.clone(),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(value),
            }))
        }
        Op::PowerLock => {
            let original = store.lock_task()?;
            if original {
                return Ok(None);
            }
            store.set_lock_task(true)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "sched".into(),
                target: "power-lock".into(),
                original: serde_json::json!(original),
                written: serde_json::json!(true),
            }))
        }
        Op::KvStr { hive, path, name, key, value } => {
            let key_ref = RegKeyRef::new(*hive, path, name);
            let original = store.kv_string(&key_ref)?;
            let next = set_kv_item(original.as_deref().unwrap_or(""), key, value);
            if original.as_deref() == Some(next.as_str()) {
                return Ok(None);
            }
            store.set_reg(&key_ref, &RegVal::Sz { value: next.clone() })?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "kvstr".into(),
                target: format!("{path}\\{name}\\{key}"),
                original: serde_json::to_value(&original)?,
                written: serde_json::json!(next),
            }))
        }
        Op::GpuSpoof { model } => {
            let gpu = hw.main_gpu.as_ref().ok_or_else(|| Error::Msg("no GPU to spoof".into()))?;
            let path = crate::items::gpu_enum_path(gpu).ok_or_else(|| Error::Msg("GPU enum path missing".into()))?;
            let key = RegKeyRef::new(crate::types::Hive::Hklm, path, "DeviceDesc");
            let original = store.get_reg(&key)?;
            let written = RegVal::Sz { value: model.clone() };
            if original.as_ref() == Some(&written) {
                return Ok(None);
            }
            store.set_reg(&key, &written)?;
            Ok(Some(BackupOp {
                item_id: item.id.clone(),
                kind: "reg".into(),
                target: format!("DeviceDesc:{}", gpu.pnp),
                original: serde_json::to_value(&original)?,
                written: serde_json::to_value(&written)?,
            }))
        }
        Op::CheckPcie | Op::CheckVc | Op::CheckXmp | Op::CheckNvAuto | Op::CacheClean => Ok(None),
    }
}

pub fn valid_game_path(path: &str) -> bool {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    GAME_EXES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

fn fail(id: &str, name: &str, message: String) -> ItemResult {
    ItemResult {
        id: id.into(),
        name: name.into(),
        ok: false,
        changed: false,
        skipped: false,
        attention: false,
        reboot: false,
        message,
    }
}

fn skip(id: &str, name: &str, message: String, attention: bool) -> ItemResult {
    ItemResult {
        id: id.into(),
        name: name.into(),
        ok: true,
        changed: false,
        skipped: true,
        attention,
        reboot: false,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use tempfile::tempdir;

    #[test]
    fn apply_game_mode_writes_and_skips_second_pass() {
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
        let first = apply(&store, &backups, &hw, req, None).unwrap();
        assert_eq!(first.failed, 0);
        assert!(first.results[0].changed);
        assert!(first.backup_file.is_some());

        let req2 = ApplyRequest {
            items: vec!["game-mode".into()],
            preset: None,
            game_path: None,
            gpu_spoof_model: None,
            risky: false,
            admin: true,
        };
        let second = apply(&store, &backups, &hw, req2, None).unwrap();
        assert!(second.results[0].skipped);
        assert!(second.backup_file.is_none());
    }

    #[test]
    fn risky_blocked_without_flag() {
        let tmp = tempdir().unwrap();
        let backups = BackupStore::open(tmp.path()).unwrap();
        let store = MemoryStore::new();
        let hw = HardwareInfo::fixture();
        let req = ApplyRequest {
            items: vec!["gpu-name-spoof".into()],
            preset: None,
            game_path: None,
            gpu_spoof_model: None,
            risky: false,
            admin: true,
        };
        let report = apply(&store, &backups, &hw, req, None).unwrap();
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn rejects_random_ifeo_exe() {
        assert!(!valid_game_path(r"C:\Windows\notepad.exe"));
        assert!(valid_game_path(r"D:\g\DeltaForceClient-Win64-Shipping.exe"));
    }
}
