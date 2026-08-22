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
use crate::store::Store;
use crate::tuning::{self, Candidate, ExperimentState};
use crate::reboot;
use crate::types::{
    ApplyReport, DetectReport, LiveMetrics, PendingRebootItem, Prefs, Preset, RebootState,
    RestoreItem, RestoreReport,
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
    let game = args.game_path.clone().unwrap_or_else(|| "-".into());
    let risky = args.risky;
    let elevated = admin();
    let req = ApplyRequest {
        items: args.items,
        preset: args.preset,
        game_path: args.game_path,
        gpu_spoof_model: args.gpu_spoof_model,
        risky,
        admin: elevated,
    };
    let report = apply::apply(store().as_ref(), &backups, &hw, req, preset_items)?;
    log::append_run(
        &paths.user,
        "apply",
        &format!(
            "apply start n={} risky={} admin={} game={}",
            report.results.len(),
            risky,
            elevated,
            game
        ),
        &report.results,
        &format!(
            "apply done ok={} fail={} skip={} attention={}",
            report.succeeded, report.failed, report.skipped, report.attention
        ),
    )?;
    let pending: Vec<PendingRebootItem> = report
        .results
        .iter()
        .filter(|r| r.reboot)
        .map(|r| PendingRebootItem {
            id: r.id.clone(),
            name: r.name.clone(),
        })
        .collect();
    reboot::add_pending(&paths.user, &pending)?;
    if !pending.is_empty() {
        log::append(
            &paths.user,
            &format!("reboot scheduled n={}", pending.len()),
        )?;
        if let Err(e) = reboot::request() {
            log::append(&paths.user, &format!("reboot fail {e}"))?;
        }
    }
    Ok(report)
}

#[tauri::command]
pub fn restore_items(items: Option<Vec<String>>) -> Result<RestoreReport> {
    let paths = paths()?;
    let backups = BackupStore::open(&paths.root)?;
    let report = restore::restore(store().as_ref(), &backups, items)?;
    log::append_run(
        &paths.user,
        "restore",
        &format!("restore start n={}", report.results.len()),
        &report.results,
        &format!(
            "restore done restored={} fail={} skip={}",
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
pub fn start_experiment(scene_id: String, game_path: Option<String>) -> Result<ExperimentState> {
    tuning::start(&paths()?.user, &scene_id, game_path)
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
    let user = paths()?.user;
    let before = tuning::load(&user)?;
    let after = tuning::confirm_round(&user, avg_fps, low_1pct, hitches)?;
    if let Some(before) = before {
        sync_experiment(&before, &after)?;
    }
    Ok(after)
}

fn group_ids(group: &str) -> Vec<String> {
    tuning::library()
        .into_iter()
        .find(|c| c.group_id == group)
        .map(|c| c.item_ids)
        .unwrap_or_default()
}

fn sync_experiment(before: &ExperimentState, after: &ExperimentState) -> Result<()> {
    if let Some(group) = &before.current_group {
        if after.rolled_back.last() == Some(group) {
            let _ = restore::restore(store().as_ref(), &BackupStore::open(&paths()?.root)?, Some(group_ids(group)));
        }
    }
    if after.current_group != before.current_group {
        if let Some(group) = &after.current_group {
            let hw = hardware::detect()?;
            let backups = BackupStore::open(&paths()?.root)?;
            let req = ApplyRequest {
                items: group_ids(group),
                preset: None,
                game_path: after.game_path.clone(),
                gpu_spoof_model: None,
                risky: false,
                admin: admin(),
            };
            let _ = apply::apply(store().as_ref(), &backups, &hw, req, None)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_experiment() -> Result<ExperimentState> {
    tuning::cancel(&paths()?.user)
}

#[tauri::command]
pub fn check_update() -> Result<UpdateInfo> {
    update::check(env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn download_update() -> Result<String> {
    let path = update::apply_latest(env!("CARGO_PKG_VERSION"))?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn diagnose() -> Result<String> {
    let hw = hardware::detect()?;
    let found = game::find_game()?;
    let store = store();
    let catalog = items::catalog(&hw, found.as_deref(), None);
    let mut lines = vec![
        format!("cpu={}", hw.cpu_name),
        format!("gpu={}", hw.main_gpu.as_ref().map(|g| g.name.as_str()).unwrap_or("none")),
        format!("brand={}", hw.brand),
        format!("admin={}", hw.is_admin),
        format!("game={}", found.as_deref().unwrap_or("none")),
        format!("scheme={:?}", Store::active_scheme(store.as_ref())?),
        format!("tool={:?}", Store::tool_scheme(store.as_ref())?),
    ];
    for item in catalog {
        let state = items::item_state(&item, store.as_ref(), &hw);
        lines.push(format!("{} state={state:?}", item.id));
    }
    Ok(lines.join("\n"))
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

#[tauri::command]
pub fn reboot_state() -> Result<RebootState> {
    let paths = paths()?;
    let hw = hardware::detect()?;
    let found = game::find_game()?;
    reboot::state(&paths.user, store().as_ref(), &hw, found.as_deref())
}

#[tauri::command]
pub fn request_reboot() -> Result<()> {
    reboot::request()
}

#[tauri::command]
pub fn ack_reboot_review() -> Result<()> {
    reboot::ack(&paths()?.user)
}
