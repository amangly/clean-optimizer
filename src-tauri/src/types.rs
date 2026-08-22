use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Hive {
    Hkcu,
    Hklm,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RegVal {
    Dword { value: u32 },
    Sz { value: String },
    Bin { value: Vec<u8> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Safe,
    Risky,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Power,
    Multi,
    Sched,
    Check,
    Cache,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub pnp: String,
    pub discrete: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub ram_gb: f64,
    pub is_laptop: bool,
    pub windows: String,
    pub is_admin: bool,
    pub gpus: Vec<GpuInfo>,
    pub main_gpu: Option<GpuInfo>,
    pub displays: Vec<String>,
    pub brand: String,
}

impl HardwareInfo {
    pub fn fixture() -> Self {
        Self {
            cpu_name: "Intel Core i7-13700K".into(),
            cpu_cores: 16,
            ram_gb: 32.0,
            is_laptop: false,
            windows: "Windows 11 26100".into(),
            is_admin: true,
            gpus: vec![GpuInfo {
                name: "NVIDIA GeForce RTX 4070".into(),
                vendor: "NVIDIA".into(),
                pnp: "PCI\\VEN_10DE&DEV_2786&SUBSYS_00000000&REV_A1\\4&1234&0&0008".into(),
                discrete: true,
            }],
            main_gpu: Some(GpuInfo {
                name: "NVIDIA GeForce RTX 4070".into(),
                vendor: "NVIDIA".into(),
                pnp: "PCI\\VEN_10DE&DEV_2786&SUBSYS_00000000&REV_A1\\4&1234&0&0008".into(),
                discrete: true,
            }),
            displays: vec!["2560x1440 @ 165Hz".into()],
            brand: "MSI".into(),
        }
    }

    pub fn main_vendor(&self) -> Option<&str> {
        self.main_gpu.as_ref().map(|g| g.vendor.as_str())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub id: String,
    pub name: String,
    pub note: String,
    pub tier: Tier,
    pub kind: ItemKind,
    pub admin: bool,
    pub default: bool,
    pub bulk_select: bool,
    pub reboot: bool,
    pub requires_game: bool,
    pub applicable: bool,
    pub optimized: bool,
    pub attention: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetectReport {
    pub hardware: HardwareInfo,
    pub game_path: Option<String>,
    pub items: Vec<ItemView>,
    pub gpu_guide: Vec<String>,
    pub spoof_models: Vec<String>,
    pub recommended_spoof: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub id: String,
    pub name: String,
    pub ok: bool,
    pub changed: bool,
    pub skipped: bool,
    pub attention: bool,
    pub reboot: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub apply_id: String,
    pub results: Vec<ItemResult>,
    pub backup_file: Option<String>,
    pub succeeded: u32,
    pub failed: u32,
    pub skipped: u32,
    pub attention: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreItem {
    pub id: String,
    pub name: String,
    pub selective: bool,
    pub conflict: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub restored: u32,
    pub failed: u32,
    pub skipped: u32,
    pub notes: Vec<String>,
    pub results: Vec<ItemResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub note: String,
    pub builtin: bool,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMetrics {
    pub cpu_pct: Option<u32>,
    pub gpu_pct: Option<u32>,
    pub ram_pct: Option<u32>,
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub fps: Option<f64>,
    pub fps_1pct: Option<f64>,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub telemetry: bool,
    pub disclaimer_accepted: bool,
    pub theme: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            telemetry: true,
            disclaimer_accepted: false,
            theme: "dark".into(),
        }
    }
}
