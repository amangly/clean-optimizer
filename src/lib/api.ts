import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyReport,
  CacheReport,
  Candidate,
  CheckResult,
  DetectReport,
  ExperimentState,
  LiveMetrics,
  Prefs,
  Preset,
  RestoreItem,
  RestoreReport,
  UpdateInfo,
} from "./types";

export function detect(gamePath?: string | null) {
  return invoke<DetectReport>("detect", { gamePath: gamePath ?? null });
}

export function applyItems(input: {
  items: string[];
  preset?: string | null;
  gamePath?: string | null;
  gpuSpoofModel?: string | null;
  risky: boolean;
}) {
  return invoke<ApplyReport>("apply_items", { args: input });
}

export function restoreItems(items?: string[] | null) {
  return invoke<RestoreReport>("restore_items", { items: items ?? null });
}

export function listRestore() {
  return invoke<RestoreItem[]>("list_restore");
}

export function listPresets() {
  return invoke<Preset[]>("list_presets");
}

export function savePreset(name: string, items: string[]) {
  return invoke<Preset>("save_preset", { name, items });
}

export function deletePreset(id: string) {
  return invoke<void>("delete_preset", { id });
}

export function findGame() {
  return invoke<string | null>("find_game");
}

export function pickGame() {
  return invoke<string | null>("pick_game");
}

export function liveMetrics() {
  return invoke<LiveMetrics>("live_metrics");
}

export function runChecks() {
  return invoke<CheckResult[]>("run_checks");
}

export function cleanShaderCache() {
  return invoke<CacheReport>("clean_shader_cache");
}

export function getPrefs() {
  return invoke<Prefs>("get_prefs");
}

export function setPrefs(next: Prefs) {
  return invoke<Prefs>("set_prefs", { next });
}

export function readLog() {
  return invoke<string>("read_log");
}

export function startExperiment(sceneId: string, gamePath?: string | null) {
  return invoke<ExperimentState>("start_experiment", { sceneId, gamePath: gamePath ?? null });
}

export function experimentStatus() {
  return invoke<ExperimentState | null>("experiment_status");
}

export function experimentLibrary() {
  return invoke<Candidate[]>("experiment_library");
}

export function confirmExperimentRound(avgFps: number, low1pct: number, hitches: number) {
  return invoke<ExperimentState>("confirm_experiment_round", { avgFps, low1pct, hitches });
}

export function cancelExperiment() {
  return invoke<ExperimentState>("cancel_experiment");
}

export function checkUpdate() {
  return invoke<UpdateInfo>("check_update");
}

export function appVersion() {
  return invoke<string>("app_version");
}

export function downloadUpdate() {
  return invoke<string>("download_update");
}

export function diagnose() {
  return invoke<string>("diagnose");
}

export function relaunchElevated() {
  return invoke<void>("relaunch_elevated");
}

export function isElevated() {
  return invoke<boolean>("is_elevated");
}

export async function closeApp() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
