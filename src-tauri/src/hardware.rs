use crate::error::Result;
use crate::types::{GpuInfo, HardwareInfo};

pub fn detect() -> Result<HardwareInfo> {
    #[cfg(windows)]
    {
        Ok(HardwareInfo {
            cpu_name: cpu_name(),
            cpu_cores: cpu_cores(),
            ram_gb: ram_gb(),
            is_laptop: is_laptop(),
            windows: windows_version(),
            is_admin: crate::win::is_admin(),
            gpus: gpus(),
            main_gpu: main_gpu(),
            displays: displays(),
            brand: brand(),
        })
    }
    #[cfg(not(windows))]
    {
        Ok(HardwareInfo::fixture())
    }
}

fn cpu_name() -> String {
    read_reg_sz(
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .unwrap_or_else(|| "Unknown CPU".into())
}

fn cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

fn ram_gb() -> f64 {
    #[cfg(windows)]
    {
        crate::metrics::total_ram_gb().unwrap_or(0.0)
    }
    #[cfg(not(windows))]
    {
        0.0
    }
}

fn is_laptop() -> bool {
    #[cfg(windows)]
    {
        let text = cmd(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).PCSystemType",
            ],
        );
        text.trim() == "2"
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn windows_version() -> String {
    let product = read_reg_sz(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "ProductName")
        .unwrap_or_else(|| "Windows".into());
    let build = read_reg_sz(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuild")
        .unwrap_or_default();
    if build.is_empty() {
        product
    } else {
        format!("{product} {build}")
    }
}

fn brand() -> String {
    cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).Manufacturer",
        ],
    )
    .trim()
    .to_string()
}

fn gpus() -> Vec<GpuInfo> {
    let raw = cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object Name,PNPDeviceId | ConvertTo-Json -Compress",
        ],
    );
    parse_gpus(&raw)
}

fn parse_gpus(raw: &str) -> Vec<GpuInfo> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).unwrap_or_default()
    } else {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .into_iter()
            .collect()
    };
    values
        .into_iter()
        .filter_map(|v| {
            let name = v.get("Name")?.as_str()?.to_string();
            let pnp = v.get("PNPDeviceId").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let vendor = vendor_of(&pnp, &name);
            Some(GpuInfo {
                discrete: vendor == "NVIDIA" || vendor == "AMD",
                name,
                vendor,
                pnp,
            })
        })
        .collect()
}

fn main_gpu() -> Option<GpuInfo> {
    let mut list = gpus();
    list.sort_by_key(|g| if g.discrete { 0 } else { 1 });
    list.into_iter().next()
}

fn vendor_of(pnp: &str, name: &str) -> String {
    let p = pnp.to_ascii_uppercase();
    let n = name.to_ascii_uppercase();
    if p.contains("VEN_10DE") || n.contains("NVIDIA") {
        "NVIDIA".into()
    } else if p.contains("VEN_1002") || n.contains("AMD") || n.contains("RADEON") {
        "AMD".into()
    } else if p.contains("VEN_8086") || n.contains("INTEL") {
        "Intel".into()
    } else {
        "Unknown".into()
    }
}

fn displays() -> Vec<String> {
    let raw = cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | ForEach-Object { '{0}x{1}' -f $_.CurrentHorizontalResolution, $_.CurrentVerticalResolution }",
        ],
    );
    raw.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "x")
        .map(str::to_string)
        .collect()
}

fn read_reg_sz(path: &str, name: &str) -> Option<String> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).ok()?;
        key.get_value(name).ok()
    }
    #[cfg(not(windows))]
    {
        let _ = (path, name);
        None
    }
}

fn cmd(bin: &str, args: &[&str]) -> String {
    #[cfg(windows)]
    {
        crate::win::hidden_command(bin)
            .args(args)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(bin)
            .args(args)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_gpu_object() {
        let raw = r#"{"Name":"NVIDIA GeForce RTX 4070","PNPDeviceId":"PCI\\VEN_10DE&DEV_2786"}"#;
        let gpus = parse_gpus(raw);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, "NVIDIA");
        assert!(gpus[0].discrete);
    }
}
