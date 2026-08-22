use crate::apply::{self, ApplyRequest};
use crate::backup::BackupStore;
use crate::cache::{self, CacheReport};
use crate::checks::{self, CheckResult};
use crate::error::Result;
use crate::game;
use crate::hardware;
use crate::items::{self, view_item};
use crate::log;
use crate::metrics;
use crate::paths::Paths;
use crate::prefs;
use crate::presets;
use crate::restore;
use crate::tuning::{self, Candidate, ExperimentState};
use crate::types::{
    ApplyReport, DetectReport, LiveMetrics, Prefs, Preset, RestoreItem, RestoreReport,
};
use crate::update::{self, UpdateInfo};
use serde::Deserialize;

fn paths() -> Result<Paths> {
    Paths::live()
}

fn store() -> Box<dyn crate::store::Store> {
    #[cfg(windows)]
    {
        Box::new(crate::win::WindowsStore)
    }
    #[cfg(not(windows))]
    {
        Box::new(crate::store::MemoryStore::new())
    }
}

fn admin() -> bool {
    #[cfg(windows)]
    {
        crate::win::is_admin()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

#[tauri::command]
pub fn detect(game_path: Option<String>) -> Result<DetectReport> {
    let hw = hardware::detect()?;
    let found = match game_path {
        Some(p) => Some(p),
        None => game::find_game()?,
    };
    let catalog = items::catalog(&hw, found.as_deref(), None);
    let store = store();
    let checks = checks::run_all();
    let item_views = catalog
        .iter()
        .map(|item| {
            let mut view = view_item(item, store.as_ref(), &hw);
            if let Some(check) = checks.iter().find(|c| c.id == item.id) {
                view.attention = check.attention;
                view.detail = check.text.clone();
                view.optimized = check.ok && !check.attention;
            }
            view
        })
        .collect();
    Ok(DetectReport {
        hardware: hw.clone(),
        game_path: found,
        items: item_views,
        gpu_guide: items::gpu_guide(&hw),
        spoof_models: items::SPOOF_MODELS.iter().map(|s| (*s).to_string()).collect(),
        recommended_spoof: items::recommended_spoof(&hw),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyArgs {
    pub items: Vec<String>,
    pub preset: Option<String>,
    pub game_path: Option<String>,
    pub gpu_spoof_model: Option<String>,
    pub risky: bool,
}

#[tauri::command]
pub fn apply_items(args: ApplyArgs) -> Result<ApplyReport> {
    let paths = paths()?;
    let backups = BackupStore::open(&paths.root)?;
    let hw = hardware::detect()?;
    let preset_items = if let Some(id) = &args.preset {
        presets::load_all(&paths.user)?
            .into_iter()
            .find(|p| p.id == *id)
            .map(|p| p.items)
    } else {
        None
    };
    let req = ApplyRequest {
        items: args.items,
        preset: args.preset,
        game_path: args.game_path,
        gpu_spoof_model: args.gpu_spoof_model,
        risky: args.risky,
        admin: admin(),
    };
    let report = apply::apply(store().as_ref(), &backups, &hw, req, preset_items)?;
    log::append(
        &paths.user,
        &format!(
            "apply ok={} fail={} skip={}",
            report.succeeded, report.failed, report.skipped
        ),
    )?;
    Ok(report)
}

#[tauri::command]
pub fn restore_items(items: Option<Vec<String>>) -> Result<RestoreReport> {
    let paths = paths()?;
    let backups = BackupStore::open(&paths.root)?;
    let report = restore::restore(store().as_ref(), &backups, items)?;
    log::append(
        &paths.user,
        &format!(
            "restore restored={} fail={} skip={}",
            report.restored, report.failed, report.skipped
        ),
    )?;
    Ok(report)
}

#[tauri::command]
pub fn list_restore() -> Result<Vec<RestoreItem>> {
    let paths = paths()?;
    let backups = BackupStore::open(&paths.root)?;
    restore::list_restore(store().as_ref(), &backups)
}

#[tauri::command]
pub fn list_presets() -> Result<Vec<Preset>> {
    presets::load_all(&paths()?.user)
}

#[tauri::command]
pub fn save_preset(name: String, items: Vec<String>) -> Result<Preset> {
    presets::save(&paths()?.user, &name, items)
}

#[tauri::command]
pub fn delete_preset(id: String) -> Result<()> {
    presets::delete(&paths()?.user, &id)
}

#[tauri::command]
pub fn find_game() -> Result<Option<String>> {
    game::find_game()
}

#[tauri::command]
pub fn pick_game() -> Result<Option<String>> {
    game::pick_game()
}

#[tauri::command]
pub fn live_metrics() -> Result<LiveMetrics> {
    metrics::snapshot()
}

#[tauri::command]
pub fn run_checks() -> Result<Vec<CheckResult>> {
    Ok(checks::run_all())
}

#[tauri::command]
pub fn clean_shader_cache() -> Result<CacheReport> {
    let report = cache::clean(&cache::shader_dirs())?;
    log::append(
        &paths()?.user,
        &format!("cache deleted={} bytes={}", report.deleted_files, report.bytes),
    )?;
    Ok(report)
}

#[tauri::command]
pub fn get_prefs() -> Result<Prefs> {
    prefs::load(&paths()?.user)
}

#[tauri::command]
pub fn set_prefs(next: Prefs) -> Result<Prefs> {
    let paths = paths()?;
    prefs::save(&paths.user, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn read_log() -> Result<String> {
    log::read_tail(&paths()?.user, 64 * 1024)
}

#[tauri::command]
pub fn start_experiment(scene_id: String) -> Result<ExperimentState> {
    tuning::start(&paths()?.user, &scene_id)
}

#[tauri::command]
pub fn experiment_status() -> Result<Option<ExperimentState>> {
    tuning::load(&paths()?.user)
}

#[tauri::command]
pub fn experiment_library() -> Result<Vec<Candidate>> {
    Ok(tuning::library())
}

#[tauri::command]
pub fn confirm_experiment_round(avg_fps: f64, low_1pct: f64, hitches: u32) -> Result<ExperimentState> {
    tuning::confirm_round(&paths()?.user, avg_fps, low_1pct, hitches)
}

#[tauri::command]
pub fn cancel_experiment() -> Result<ExperimentState> {
    tuning::cancel(&paths()?.user)
}

#[tauri::command]
pub fn check_update() -> Result<UpdateInfo> {
    update::check("0.1.0", None)
}

#[tauri::command]
pub fn relaunch_elevated() -> Result<()> {
    #[cfg(windows)]
    {
        crate::win::relaunch_elevated()
    }
    #[cfg(not(windows))]
    {
        Err(crate::error::Error::Msg("Windows only".into()))
    }
}

#[tauri::command]
pub fn is_elevated() -> bool {
    admin()
}
