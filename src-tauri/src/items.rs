use crate::store::{get_kv_item, RegKeyRef, Store, TOOL_SCHEME_NAME, ULTIMATE_TEMPLATE};
use crate::types::{HardwareInfo, Hive, ItemKind, ItemView, RegVal, Tier};

pub const SELECTIVE_RESTORE: &[&str] = &[
    "game-mode",
    "dvr-off",
    "prio-separation",
    "net-throttling-off",
    "sys-responsiveness",
    "mmcss-games",
    "fso-off",
    "gpu-pref",
];

pub const GAME_EXES: &[&str] = &[
    "DeltaForceClient-Win64-Shipping.exe",
    "DeltaForceClient.exe",
    "DeltaForce.exe",
];

pub const SPOOF_MODELS: &[&str] = &[
    "NVIDIA GeForce GTX 750 Ti",
    "NVIDIA GeForce GTX 1050 Ti",
    "NVIDIA GeForce RTX 2050",
    "NVIDIA GeForce RTX 2060",
    "AMD Radeon RX560",
];

pub const SUB_USB: &str = "2a737441-1930-4402-8d77-b2bebba308a3";
pub const SUB_PROC: &str = "54533251-82be-4824-96c1-47b60b740d00";
pub const USB_LINK: &str = "d4e98f31-5ffe-4ce1-be31-1b38b384c009";
pub const PROC_CHECK: &str = "4d2b0152-7d5c-498b-88e2-34345392a2c5";
pub const HETERO: &str = "93b8b6dc-0698-4d1c-9ee4-0644e900c85d";
pub const SHORT_HETERO: &str = "bae08b81-2d5e-4688-ad6a-13243356654b";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Reg {
        hive: Hive,
        path: String,
        name: String,
        value: RegVal,
    },
    PowerUltimate,
    PowerCfg {
        sub: String,
        setting: String,
        value: u32,
        optional: bool,
    },
    MmAgent {
        feature: String,
        enabled: bool,
    },
    HibernateOff,
    Bcd {
        name: String,
        value: String,
    },
    PowerLock,
    KvStr {
        hive: Hive,
        path: String,
        name: String,
        key: String,
        value: String,
    },
    GpuSpoof {
        model: String,
    },
    CheckPcie,
    CheckVc,
    CheckXmp,
    CheckNvAuto,
    CacheClean,
}

#[derive(Clone, Debug)]
pub struct OptItem {
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
    pub ops: Vec<Op>,
}

impl OptItem {
    fn base(
        id: &str,
        name: &str,
        note: &str,
        kind: ItemKind,
        admin: bool,
        default: bool,
        reboot: bool,
        ops: Vec<Op>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            note: note.into(),
            tier: Tier::Safe,
            kind,
            admin,
            default,
            bulk_select: true,
            reboot,
            requires_game: false,
            ops,
        }
    }
}

fn reg(hive: Hive, path: &str, name: &str, value: RegVal) -> Op {
    Op::Reg {
        hive,
        path: path.into(),
        name: name.into(),
        value,
    }
}

fn dword(v: u32) -> RegVal {
    RegVal::Dword { value: v }
}

fn sz(v: &str) -> RegVal {
    RegVal::Sz { value: v.into() }
}

pub fn recommended_spoof(hw: &HardwareInfo) -> Option<String> {
    let vendor = hw.main_vendor()?;
    if vendor.eq_ignore_ascii_case("NVIDIA") || vendor.eq_ignore_ascii_case("AMD") {
        Some(default_spoof_model(vendor, hw.is_laptop))
    } else {
        None
    }
}

pub fn display_name(id: &str) -> String {
    let hw = HardwareInfo::fixture();
    catalog(
        &hw,
        Some(r"C:\Delta Force\DeltaForceClient-Win64-Shipping.exe"),
        None,
    )
    .into_iter()
    .find(|item| item.id == id)
    .map(|item| item.name)
    .unwrap_or_else(|| id.to_string())
}

pub fn pick_irq_mask(cores: &[(u8, u64)]) -> Option<u64> {
    if cores.len() < 2 {
        return None;
    }
    let top = cores.iter().map(|c| c.0).max()?;
    let mut cand: Vec<u64> = cores.iter().filter(|c| c.0 == top).map(|c| c.1).collect();
    if cand.len() < 2 {
        return None;
    }
    cand.sort_unstable();
    let mask = *cand.last()?;
    if mask & 1 != 0 {
        return None;
    }
    Some(mask)
}

fn host_irq_mask() -> Option<u64> {
    #[cfg(windows)]
    {
        pick_irq_mask(&crate::win_cpu::processor_cores()?)
    }
    #[cfg(not(windows))]
    {
        let _ = pick_irq_mask;
        None
    }
}

pub fn gpu_class_path(gpu: &crate::types::GpuInfo) -> Option<String> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let enum_key = hklm
            .open_subkey(format!(r"SYSTEM\CurrentControlSet\Enum\{}", gpu.pnp))
            .ok()?;
        let driver: String = enum_key.get_value("Driver").ok()?;
        let prefix = "{4d36e968-e325-11ce-bfc1-08002be10318}\\";
        if !driver.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) {
            return None;
        }
        let class = format!(r"SYSTEM\CurrentControlSet\Control\Class\{driver}");
        hklm.open_subkey(&class).ok()?;
        Some(class)
    }
    #[cfg(not(windows))]
    {
        let _ = gpu;
        None
    }
}

fn gpu_irq_ops(hw: &HardwareInfo) -> Option<Vec<Op>> {
    if hw.cpu_cores < 4 || hw.cpu_cores > 64 {
        return None;
    }
    let gpu = hw.main_gpu.as_ref()?;
    if gpu.pnp.is_empty() {
        return None;
    }
    let nvidia = hw.gpus.iter().filter(|g| g.vendor == "NVIDIA").count();
    if gpu.vendor == "NVIDIA" && nvidia > 1 {
        return None;
    }
    let mask = host_irq_mask()?;
    let path = format!(
        r"SYSTEM\CurrentControlSet\Enum\{}\Device Parameters\Interrupt Management\Affinity Policy",
        gpu.pnp
    );
    Some(vec![
        reg(Hive::Hklm, &path, "DevicePolicy", dword(4)),
        Op::Reg {
            hive: Hive::Hklm,
            path,
            name: "AssignmentSetOverride".into(),
            value: RegVal::Bin {
                value: mask.to_le_bytes().to_vec(),
            },
        },
    ])
}

pub fn default_spoof_model(vendor: &str, laptop: bool) -> String {
    if vendor.eq_ignore_ascii_case("AMD") {
        return "AMD Radeon RX560".into();
    }
    if laptop {
        "NVIDIA GeForce GTX 1050 Ti".into()
    } else {
        "NVIDIA GeForce GTX 750 Ti".into()
    }
}

pub fn gpu_enum_path(gpu: &crate::types::GpuInfo) -> Option<String> {
    let pnp = gpu.pnp.trim();
    if pnp.is_empty() {
        return None;
    }
    Some(format!(r"SYSTEM\CurrentControlSet\Enum\{pnp}"))
}

pub fn catalog(hw: &HardwareInfo, game_path: Option<&str>, spoof: Option<&str>) -> Vec<OptItem> {
    let mut items = Vec::new();

    items.push(OptItem::base(
        "power-ultimate",
        "Switch the power plan to Ultimate Performance",
        "Removes conservative CPU frequency caps. Desktops gain the most. Laptops lose battery life. Needs a reboot to settle.",
        ItemKind::Power,
        true,
        true,
        true,
        vec![Op::PowerUltimate],
    ));

    items.push(OptItem::base(
        "power-tuning",
        "Tune hidden power-plan settings",
        "Turns off USB 3 link power saving, sets the processor check interval to 5000 ms, prefers P-cores on hybrid CPUs, and disables power throttling. Missing hybrid settings are skipped.",
        ItemKind::Multi,
        true,
        true,
        true,
        vec![
            Op::PowerCfg { sub: SUB_USB.into(), setting: USB_LINK.into(), value: 0, optional: false },
            Op::PowerCfg { sub: SUB_PROC.into(), setting: PROC_CHECK.into(), value: 5000, optional: false },
            Op::PowerCfg { sub: SUB_PROC.into(), setting: HETERO.into(), value: 1, optional: true },
            Op::PowerCfg { sub: SUB_PROC.into(), setting: SHORT_HETERO.into(), value: 1, optional: true },
            reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling", "PowerThrottlingOff", dword(1)),
        ],
    ));

    items.push(OptItem::base(
        "powerplan-lock",
        "Lock the power plan",
        "Creates a scheduled task that writes the current plan back every minute. Delta Force has been seen changing the plan at launch. Restore deletes the task.",
        ItemKind::Sched,
        true,
        false,
        false,
        vec![Op::PowerLock],
    ));

    items.push(OptItem::base(
        "hags",
        "Turn on Hardware-accelerated GPU Scheduling",
        "Lowers GPU scheduling latency on Windows 10 2004+ with a current driver. Needs a reboot.",
        ItemKind::Multi,
        true,
        true,
        true,
        vec![reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", dword(2))],
    ));

    items.push(OptItem::base(
        "game-mode",
        "Turn on Windows Game Mode",
        "Windows lowers background work while a game is in the foreground.",
        ItemKind::Multi,
        false,
        true,
        false,
        vec![
            reg(Hive::Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled", dword(1)),
            reg(Hive::Hkcu, r"Software\Microsoft\GameBar", "AllowAutoGameMode", dword(1)),
        ],
    ));

    items.push(OptItem::base(
        "dvr-off",
        "Turn off Xbox background recording",
        "Game DVR keeps the GPU encoder and memory bandwidth busy. That is a common hidden hitch source.",
        ItemKind::Multi,
        false,
        true,
        false,
        vec![
            reg(Hive::Hkcu, r"System\GameConfigStore", "GameDVR_Enabled", dword(0)),
            reg(Hive::Hkcu, r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", dword(0)),
        ],
    ));

    items.push(OptItem::base(
        "prio-separation",
        "Raise foreground scheduling weight",
        "Sets Win32PrioritySeparation to 40: short, fixed quanta with more time for the foreground process.",
        ItemKind::Multi,
        true,
        true,
        false,
        vec![reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Control\PriorityControl", "Win32PrioritySeparation", dword(40))],
    ));

    items.push(OptItem::base(
        "paging-exec",
        "Keep kernel code in RAM",
        "Sets DisablePagingExecutive so kernel pages stay resident. Skip this on machines with 8 GB or less.",
        ItemKind::Multi,
        true,
        true,
        true,
        vec![reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management", "DisablePagingExecutive", dword(1))],
    ));

    items.push(OptItem::base(
        "wer-off",
        "Turn off Windows Error Reporting",
        "Stops a dump collection stall right as the game crashes.",
        ItemKind::Multi,
        false,
        true,
        false,
        vec![reg(Hive::Hkcu, r"Software\Microsoft\Windows\Windows Error Reporting", "Disabled", dword(1))],
    ));

    let mut mem = OptItem::base(
        "mem-compress-off",
        "Turn off memory compression and page combining",
        "Saves the CPU work of compress/decompress. Memory pressure hits the page file sooner. Manual only. Not in Select all or any built-in preset.",
        ItemKind::Multi,
        true,
        false,
        true,
        vec![
            Op::MmAgent { feature: "mc".into(), enabled: false },
            Op::MmAgent { feature: "pc".into(), enabled: false },
        ],
    );
    mem.bulk_select = false;
    items.push(mem);

    items.push(OptItem::base(
        "transparency-off",
        "Turn off window transparency",
        "Cuts DWM glass cost. Small gain on low-end machines.",
        ItemKind::Multi,
        false,
        true,
        false,
        vec![reg(Hive::Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency", dword(0))],
    ));

    items.push(OptItem::base(
        "visualfx-perf",
        "Set visual effects to best performance",
        "Turns off window animations and shadows. The desktop looks plain. Off by default.",
        ItemKind::Multi,
        false,
        false,
        false,
        vec![reg(Hive::Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects", "VisualFXSetting", dword(2))],
    ));

    items.push(OptItem::base(
        "mouse-accel-off",
        "Turn off Enhance pointer precision",
        "Linear mouse response. Changes aim feel. Off by default.",
        ItemKind::Multi,
        false,
        false,
        false,
        vec![
            reg(Hive::Hkcu, r"Control Panel\Mouse", "MouseSpeed", sz("0")),
            reg(Hive::Hkcu, r"Control Panel\Mouse", "MouseThreshold1", sz("0")),
            reg(Hive::Hkcu, r"Control Panel\Mouse", "MouseThreshold2", sz("0")),
        ],
    ));

    items.push(OptItem::base(
        "mpo-off",
        "Disable Multiplane Overlay",
        "MPO plus some drivers causes flicker and hitches. NVIDIA shipped a disable tool for this. Video playback uses a bit more DWM power. Needs a reboot.",
        ItemKind::Multi,
        true,
        true,
        true,
        vec![reg(Hive::Hklm, r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", dword(5))],
    ));

    let mmcss = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
    items.push(OptItem::base(
        "net-throttling-off",
        "Remove multimedia network throttling",
        "Windows defaults to 10 packets/ms for non-multimedia traffic. 0xFFFFFFFF removes the cap.",
        ItemKind::Multi,
        true,
        true,
        false,
        vec![reg(Hive::Hklm, mmcss, "NetworkThrottlingIndex", dword(u32::MAX))],
    ));

    items.push(OptItem::base(
        "sys-responsiveness",
        "Set MMCSS background reserve to 10%",
        "10 is the lowest value Windows honors. Values below 10 get clamped to 20.",
        ItemKind::Multi,
        true,
        true,
        false,
        vec![reg(Hive::Hklm, mmcss, "SystemResponsiveness", dword(10))],
    ));

    items.push(OptItem::base(
        "sysmain-off",
        "Disable the SysMain prefetch service",
        "SysMain (Superfetch) prefetches in the background. Cold starts can get slower. Off by default. Needs a reboot.",
        ItemKind::Multi,
        true,
        false,
        true,
        vec![reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Services\SysMain", "Start", dword(4))],
    ));

    items.push(OptItem::base(
        "wsearch-off",
        "Disable Windows Search indexing",
        "The indexer walks disks in the background. Start menu and Explorer search get slower. Off by default.",
        ItemKind::Multi,
        true,
        false,
        true,
        vec![reg(Hive::Hklm, r"SYSTEM\CurrentControlSet\Services\WSearch", "Start", dword(4))],
    ));

    items.push(OptItem::base(
        "hibernate-off",
        "Turn off hibernate and Fast Startup",
        "Deletes hiberfil.sys and the fake-shutdown Fast Startup state. Laptops then only sleep on lid close. Default on desktops.",
        ItemKind::Multi,
        true,
        !hw.is_laptop,
        false,
        vec![Op::HibernateOff],
    ));

    if let Some(gpu) = &hw.main_gpu {
        let class_ops = gpu_class_path(gpu)
            .map(|class| vec![reg(Hive::Hklm, &class, "DisableDynamicPstate", dword(1))])
            .unwrap_or_default();
        items.push(OptItem::base(
            "gpu-pstate-lock",
            "Stop the GPU from dropping P-states",
            "Writes DisableDynamicPstate on the display-adapter class key that Enum\\Driver points at. Idle power and heat go up. Off by default. Needs a reboot. Skipped if that class instance cannot be resolved.",
            ItemKind::Multi,
            true,
            false,
            true,
            class_ops,
        ));
    }

    items.push(OptItem::base(
        "nv-autoopt-off",
        "Check NVIDIA App automatic optimize",
        "Read-only. If NVIDIA App is still auto-applying Optimal Playable Settings, turn that off inside the App. This item does not write the XML.",
        ItemKind::Check,
        false,
        false,
        false,
        vec![Op::CheckNvAuto],
    ));

    items.push(OptItem::base(
        "gpu-irq-affinity",
        "Pin GPU interrupts to a P-core",
        "DevicePolicy=4 plus a KAFFINITY mask on the last performance core. Skipped if the core map cannot be read, if only one P-core exists, or if the mask includes CPU0. Needs a reboot.",
        ItemKind::Multi,
        true,
        false,
        true,
        gpu_irq_ops(hw).unwrap_or_default(),
    ));

    let mm_tasks = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games";
    items.push(OptItem::base(
        "mmcss-games",
        "Raise the MMCSS Games task",
        "Sets GPU priority 8, task priority 6, scheduling and SFIO High. Small gain, full restore.",
        ItemKind::Multi,
        true,
        true,
        false,
        vec![
            reg(Hive::Hklm, mm_tasks, "GPU Priority", dword(8)),
            reg(Hive::Hklm, mm_tasks, "Priority", dword(6)),
            reg(Hive::Hklm, mm_tasks, "Scheduling Category", sz("High")),
            reg(Hive::Hklm, mm_tasks, "SFIO Priority", sz("High")),
        ],
    ));

    items.push(OptItem::base(
        "windowed-opt-off",
        "Turn off optimizations for windowed games",
        "Edits only SwapEffectUpgradeEnable in the DirectX composite string. Microsoft and the community disagree on hitching. Off by default.",
        ItemKind::Multi,
        false,
        false,
        false,
        vec![Op::KvStr {
            hive: Hive::Hkcu,
            path: r"Software\Microsoft\DirectX\UserGpuPreferences".into(),
            name: "DirectXUserGlobalSettings".into(),
            key: "SwapEffectUpgradeEnable".into(),
            value: "0".into(),
        }],
    ));

    items.push(OptItem::base(
        "pcie-check",
        "Check the PCIe link",
        "Read-only. An x8 or x4 cap usually means the wrong slot or a cheap riser. Idle downclocking is normal.",
        ItemKind::Check,
        false,
        false,
        false,
        vec![Op::CheckPcie],
    ));

    items.push(OptItem::base(
        "vcredist-check",
        "Check VC++ v14 runtimes",
        "Read-only. Reports a problem only if x64 or x86 VC++ 2015-2022 is missing. Version skew between the two is common and is not a fail.",
        ItemKind::Check,
        false,
        true,
        false,
        vec![Op::CheckVc],
    ));

    items.push(OptItem::base(
        "xmp-check",
        "Check memory frequency",
        "Read-only. Compares the running frequency to the SMBIOS rated speed and names the BIOS menu for this board (XMP, A-XMP, EXPO, DOCP).",
        ItemKind::Check,
        false,
        false,
        false,
        vec![Op::CheckXmp],
    ));

    let mut cache = OptItem::base(
        "shader-cache-clean",
        "Clear shader caches",
        "Deletes driver and DirectX shader caches only. First matches after this will hitch while shaders rebuild. No backup. Restore will not put the files back.",
        ItemKind::Cache,
        false,
        false,
        false,
        vec![Op::CacheClean],
    );
    cache.bulk_select = false;
    items.push(cache);

    items.push(OptItem::base(
        "dyntick-off",
        "Disable dynamic tick",
        "bcdedit disabledynamictick yes. Some machines get steadier frame times. Idle power rises. Off by default. Needs a reboot.",
        ItemKind::Multi,
        true,
        false,
        true,
        vec![Op::Bcd { name: "disabledynamictick".into(), value: "yes".into() }],
    ));

    if let Some(path) = game_path {
        let exe = std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut fso = OptItem::base(
            "fso-off",
            "Disable fullscreen optimizations for the game",
            "Writes the AppCompat flag so the game can take exclusive fullscreen. Needs the game exe path.",
            ItemKind::Multi,
            false,
            true,
            false,
            vec![reg(
                Hive::Hkcu,
                r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers",
                path,
                sz("~ DISABLEDXMAXIMIZEDWINDOWEDMODE"),
            )],
        );
        fso.requires_game = true;
        items.push(fso);

        let mut gpu_pref = OptItem::base(
            "gpu-pref",
            "Force the game onto the high-performance GPU",
            "Needed on laptops with iGPU + dGPU. Needs the game exe path.",
            ItemKind::Multi,
            false,
            true,
            false,
            vec![reg(
                Hive::Hkcu,
                r"Software\Microsoft\DirectX\UserGpuPreferences",
                path,
                sz("GpuPreference=2;"),
            )],
        );
        gpu_pref.requires_game = true;
        items.push(gpu_pref);

        let ifeo = format!(
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\{exe}\PerfOptions"
        );
        let mut prio = OptItem::base(
            "game-priority",
            "Raise the game process CPU and IO priority",
            "IFEO CpuPriorityClass and IoPriority = 3 (high). Only the Delta Force exe names are allowed.",
            ItemKind::Multi,
            true,
            true,
            false,
            vec![
                reg(Hive::Hklm, &ifeo, "CpuPriorityClass", dword(3)),
                reg(Hive::Hklm, &ifeo, "IoPriority", dword(3)),
            ],
        );
        prio.requires_game = true;
        items.push(prio);
    } else {
        for (id, name, note) in [
            ("fso-off", "Disable fullscreen optimizations for the game", "Needs the game exe path."),
            ("gpu-pref", "Force the game onto the high-performance GPU", "Needs the game exe path."),
            ("game-priority", "Raise the game process CPU and IO priority", "Needs the game exe path."),
        ] {
            let mut item = OptItem::base(id, name, note, ItemKind::Multi, id == "game-priority", true, false, vec![]);
            item.requires_game = true;
            items.push(item);
        }
    }

    if let Some(gpu) = &hw.main_gpu {
        if gpu.vendor == "NVIDIA" || gpu.vendor == "AMD" {
            let model = spoof
                .filter(|m| SPOOF_MODELS.contains(m))
                .map(|s| s.to_string())
                .unwrap_or_else(|| default_spoof_model(&gpu.vendor, hw.is_laptop));
            let path = gpu_enum_path(gpu).unwrap_or_else(|| r"SYSTEM\CurrentControlSet\Enum\PCI\VEN_10DE".into());
            let mut spoof_item = OptItem::base(
                "gpu-name-spoof",
                "Spoof the reported GPU name",
                "Rewrites PCI DeviceDesc so the game may pick a different render path. Some machines lose frames. A driver install writes the real name back. Anti-cheat treatment of a lying DeviceDesc is unpublished.",
                ItemKind::Multi,
                true,
                false,
                false,
                vec![Op::GpuSpoof { model }],
            );
            spoof_item.tier = Tier::Risky;
            let _ = path;
            items.push(spoof_item);
        }
    }

    items
}

pub fn view_item(item: &OptItem, store: &dyn Store, hw: &HardwareInfo) -> ItemView {
    let (optimized, attention, detail) = match item.kind {
        ItemKind::Check => (false, false, String::new()),
        ItemKind::Cache => (false, false, "No backup. Cache rebuilds itself.".into()),
        _ => match item_state(item, store, hw) {
            Ok(true) => (true, false, "Matches the target.".into()),
            Ok(false) => (false, false, String::new()),
            Err(e) => (false, false, e),
        },
    };
    ItemView {
        id: item.id.clone(),
        name: item.name.clone(),
        note: item.note.clone(),
        tier: item.tier,
        kind: item.kind,
        admin: item.admin,
        default: item.default,
        bulk_select: item.bulk_select,
        reboot: item.reboot,
        requires_game: item.requires_game,
        applicable: !item.ops.is_empty() || matches!(item.kind, ItemKind::Check | ItemKind::Cache),
        optimized,
        attention,
        detail,
    }
}

pub fn item_state(item: &OptItem, store: &dyn Store, hw: &HardwareInfo) -> std::result::Result<bool, String> {
    if item.ops.is_empty() {
        return Ok(false);
    }
    for op in &item.ops {
        if !op_matches(op, store, hw).map_err(|e| e.to_string())? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn op_matches(op: &Op, store: &dyn Store, hw: &HardwareInfo) -> crate::error::Result<bool> {
    match op {
        Op::Reg { hive, path, name, value } => {
            let key = RegKeyRef::new(*hive, path, name);
            Ok(store.get_reg(&key)? == Some(value.clone()))
        }
        Op::PowerUltimate => {
            let active = store.active_scheme()?;
            if let Some(id) = &active {
                if crate::store::guid_eq(id, ULTIMATE_TEMPLATE) {
                    return Ok(true);
                }
                if let Some(tool) = store.tool_scheme()? {
                    if crate::store::guid_eq(id, &tool) {
                        return Ok(true);
                    }
                }
                let _ = TOOL_SCHEME_NAME;
            }
            Ok(false)
        }
        Op::PowerCfg { sub, setting, value, optional } => match store.power_setting(sub, setting)? {
            Some(v) => Ok(v == *value),
            None => Ok(*optional),
        },
        Op::MmAgent { feature, enabled } => Ok(store.mmagent(feature)? == Some(*enabled)),
        Op::HibernateOff => Ok(store.hibernate()? == Some(false)),
        Op::Bcd { name, value } => Ok(store
            .bcd(name)?
            .as_deref()
            .is_some_and(|got| got.eq_ignore_ascii_case(value))),
        Op::PowerLock => store.lock_task(),
        Op::KvStr { hive, path, name, key, value } => {
            let key_ref = RegKeyRef::new(*hive, path, name);
            let raw = store.kv_string(&key_ref)?.unwrap_or_default();
            Ok(get_kv_item(&raw, key).as_deref() == Some(value.as_str()))
        }
        Op::GpuSpoof { model } => {
            if let Some(gpu) = &hw.main_gpu {
                if let Some(path) = gpu_enum_path(gpu) {
                    let key = RegKeyRef::new(Hive::Hklm, path, "DeviceDesc");
                    return Ok(store.get_reg(&key)? == Some(sz(model)));
                }
            }
            Ok(false)
        }
        Op::CheckPcie | Op::CheckVc | Op::CheckXmp | Op::CheckNvAuto | Op::CacheClean => Ok(false),
    }
}

pub fn gpu_guide(hw: &HardwareInfo) -> Vec<String> {
    let vendor = hw.main_vendor().unwrap_or("Unknown");
    let name = hw
        .main_gpu
        .as_ref()
        .map(|g| g.name.as_str())
        .unwrap_or("no discrete GPU");
    let mut lines = vec![format!("Main GPU: {name} ({vendor}). Set these in the vendor panel. This app does not import driver profiles.")];
    match vendor {
        "NVIDIA" => {
            lines.push("Power management: Prefer maximum performance.".into());
            lines.push("Low Latency Mode: Ultra, or Reflex if the game exposes it.".into());
            lines.push("Vertical sync: Off. Cap the frame rate in-game.".into());
            lines.push("Threaded optimization: On.".into());
            if name.contains("RTX") {
                lines.push("DLSS: Use if the game offers it. Preset K is 40/50-series.".into());
            }
            lines.push("Turn off NVIDIA App automatic optimize if it is on.".into());
        }
        "AMD" => {
            lines.push("Radeon Anti-Lag: On.".into());
            lines.push("Radeon Chill: Off.".into());
            lines.push("Wait for Vertical Refresh: Always off.".into());
            lines.push("FSR: Use if the game offers it.".into());
        }
        "Intel" => {
            lines.push("XeSS: Use if the game offers it.".into());
            lines.push("VSync: Off.".into());
        }
        _ => lines.push("Open the GPU control panel for this adapter and set a high-performance profile.".into()),
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;

    #[test]
    fn catalog_contains_core_ids() {
        let hw = HardwareInfo::fixture();
        let items = catalog(&hw, Some(r"D:\Delta Force\DeltaForceClient-Win64-Shipping.exe"), None);
        for id in [
            "power-ultimate", "hags", "game-mode", "dvr-off", "gpu-name-spoof",
            "fso-off", "shader-cache-clean", "vcredist-check",
        ] {
            assert!(items.iter().any(|i| i.id == id), "missing {id}");
        }
        let spoof = items.iter().find(|i| i.id == "gpu-name-spoof").unwrap();
        assert_eq!(spoof.tier, Tier::Risky);
    }

    #[test]
    fn spoof_defaults() {
        assert_eq!(default_spoof_model("NVIDIA", true), "NVIDIA GeForce GTX 1050 Ti");
        assert_eq!(default_spoof_model("NVIDIA", false), "NVIDIA GeForce GTX 750 Ti");
        assert_eq!(default_spoof_model("AMD", false), "AMD Radeon RX560");
    }

    #[test]
    fn catalog_has_every_public_id() {
        let hw = HardwareInfo::fixture();
        let items = catalog(&hw, Some(r"D:\Delta Force\DeltaForceClient-Win64-Shipping.exe"), None);
        for id in [
            "power-ultimate", "power-tuning", "powerplan-lock", "hags", "game-mode", "dvr-off",
            "prio-separation", "paging-exec", "wer-off", "mem-compress-off", "transparency-off",
            "visualfx-perf", "mouse-accel-off", "mpo-off", "net-throttling-off", "sys-responsiveness",
            "sysmain-off", "wsearch-off", "hibernate-off", "gpu-pstate-lock", "nv-autoopt-off",
            "gpu-irq-affinity", "mmcss-games", "windowed-opt-off", "pcie-check", "vcredist-check",
            "xmp-check", "shader-cache-clean", "dyntick-off", "fso-off", "gpu-pref", "game-priority",
            "gpu-name-spoof",
        ] {
            assert!(items.iter().any(|i| i.id == id), "missing {id}");
        }
        assert_eq!(items.len(), 33);
    }

    #[test]
    fn irq_mask_picks_last_pcore() {
        assert_eq!(pick_irq_mask(&[(0, 1), (2, 0x10), (2, 0x20)]), Some(0x20));
        assert_eq!(pick_irq_mask(&[(2, 1), (2, 3)]), None);
        assert_eq!(pick_irq_mask(&[(2, 4)]), None);
    }

    #[test]
    fn irq_placeholder_path_is_gone() {
        let hw = HardwareInfo::fixture();
        let items = catalog(&hw, None, None);
        let irq = items.iter().find(|i| i.id == "gpu-irq-affinity").unwrap();
        for op in &irq.ops {
            if let Op::Reg { path, .. } = op {
                assert!(!path.contains("VEN_PLACEHOLDER"), "{path}");
            }
        }
    }

    #[test]
    fn gpu_pstate_does_not_hardcode_slot_zero() {
        let hw = HardwareInfo::fixture();
        let items = catalog(&hw, None, None);
        let item = items.iter().find(|i| i.id == "gpu-pstate-lock").unwrap();
        assert!(item.ops.is_empty(), "fixture GPU has no live Enum Driver mapping");
        assert!(gpu_class_path(hw.main_gpu.as_ref().unwrap()).is_none());
    }

    #[test]
    fn detect_game_mode_false_until_written() {
        let hw = HardwareInfo::fixture();
        let store = MemoryStore::new();
        let items = catalog(&hw, None, None);
        let gm = items.iter().find(|i| i.id == "game-mode").unwrap();
        assert!(!item_state(gm, &store, &hw).unwrap());
    }

    #[test]
    fn bcd_match_ignores_case() {
        let hw = HardwareInfo::fixture();
        let store = MemoryStore::new();
        store.set_bcd("disabledynamictick", "Yes").unwrap();
        let op = Op::Bcd {
            name: "disabledynamictick".into(),
            value: "yes".into(),
        };
        assert!(op_matches(&op, &store, &hw).unwrap());
    }
}
