mod apply;
mod backup;
mod cache;
mod checks;
mod commands;
mod error;
mod game;
mod hardware;
mod items;
mod log;
mod metrics;
mod paths;
mod prefs;
mod presets;
mod reboot;
mod restore;
mod store;
mod tuning;
mod types;
mod update;

#[cfg(windows)]
mod win;
#[cfg(windows)]
mod win_cpu;

pub use commands::*;

use tauri::webview::PageLoadEvent;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

fn external_navigation_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1")
            );
            let is_internal = url.scheme() == "tauri" || is_internal_host;
            if is_internal {
                return true;
            }
            let is_external_link = matches!(url.scheme(), "http" | "https" | "mailto" | "tel");
            if is_external_link {
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }
            true
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    crate::win::kill_other_instances();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(external_navigation_plugin())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = webview.app_handle().get_webview_window("main").map(|w| {
                    let _ = w.show();
                    let _ = w.set_focus();
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            detect,
            apply_items,
            restore_items,
            list_restore,
            list_presets,
            save_preset,
            delete_preset,
            find_game,
            pick_game,
            live_metrics,
            run_checks,
            clean_shader_cache,
            get_prefs,
            set_prefs,
            read_log,
            start_experiment,
            experiment_status,
            experiment_library,
            confirm_experiment_round,
            cancel_experiment,
            check_update,
            app_version,
            download_update,
            diagnose,
            relaunch_elevated,
            is_elevated,
            reboot_state,
            request_reboot,
            ack_reboot_review
        ])
        .run(tauri::generate_context!())
        .expect("error while running Clean Optimizer");
}
