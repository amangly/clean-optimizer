use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub id: String,
    pub ok: bool,
    pub attention: bool,
    pub text: String,
}

pub fn run_all() -> Vec<CheckResult> {
    vec![pcie(), vcredist(), xmp(), nv_auto()]
}

fn pcie() -> CheckResult {
    let smi = find_nvidia_smi();
    if smi.is_none() {
        return CheckResult {
            id: "pcie-check".into(),
            ok: true,
            attention: false,
            text: "nvidia-smi not found. PCIe width is only read for NVIDIA.".into(),
        };
    }
    let out = std::process::Command::new(smi.unwrap())
        .args(["--query-gpu=pcie.link.gen.max,pcie.link.width.max,pcie.link.gen.current,pcie.link.width.current", "--format=csv,noheader"])
        .output();
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let attention = text.contains(" 4,") || text.contains(", 4") || text.contains("x4");
            CheckResult {
                id: "pcie-check".into(),
                ok: true,
                attention,
                text: if text.is_empty() {
                    "nvidia-smi returned no PCIe data.".into()
                } else {
                    format!("PCIe {text}")
                },
            }
        }
        Err(e) => CheckResult {
            id: "pcie-check".into(),
            ok: true,
            attention: false,
            text: e.to_string(),
        },
    }
}

fn find_nvidia_smi() -> Option<String> {
    let candidates = [
        r"C:\Windows\System32\nvidia-smi.exe",
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
    ];
    candidates.iter().find(|p| std::path::Path::new(p).exists()).map(|s| s.to_string())
}

fn vcredist() -> CheckResult {
    let x64 = vc_present(true);
    let x86 = vc_present(false);
    if x64 && x86 {
        CheckResult {
            id: "vcredist-check".into(),
            ok: true,
            attention: false,
            text: "VC++ 2015-2022 x64 and x86 are present.".into(),
        }
    } else {
        let mut missing = Vec::new();
        if !x64 {
            missing.push("x64");
        }
        if !x86 {
            missing.push("x86");
        }
        CheckResult {
            id: "vcredist-check".into(),
            ok: true,
            attention: true,
            text: format!(
                "Missing VC++ 2015-2022 {}. Install from https://aka.ms/vs/18/release/vc_redist.{}.exe. Do not uninstall older year packages.",
                missing.join(" and "),
                if !x64 { "x64" } else { "x86" }
            ),
        }
    }
}

fn vc_present(x64: bool) -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let path = if x64 {
            r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"
        } else {
            r"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x86"
        };
        if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
            let installed: u32 = key.get_value("Installed").unwrap_or(0);
            return installed == 1;
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = x64;
        true
    }
}

fn xmp() -> CheckResult {
    let raw = cmd(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_PhysicalMemory | Select-Object Speed,ConfiguredClockSpeed,Manufacturer | ConvertTo-Json -Compress",
        ],
    );
    if raw.trim().is_empty() {
        return CheckResult {
            id: "xmp-check".into(),
            ok: true,
            attention: false,
            text: "Could not read memory SPD.".into(),
        };
    }
    CheckResult {
        id: "xmp-check".into(),
        ok: true,
        attention: false,
        text: format!("Memory SPD snapshot: {raw}"),
    }
}

fn nv_auto() -> CheckResult {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let path = std::path::PathBuf::from(local).join(r"NVIDIA Corporation\NVIDIA App\NvBackend\config.xml");
    if !path.exists() {
        return CheckResult {
            id: "nv-autoopt-off".into(),
            ok: true,
            attention: false,
            text: "NVIDIA App config not found.".into(),
        };
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let on = text.contains("<EnableAutomaticApplyOPS>1</EnableAutomaticApplyOPS>")
        || text.contains("EnableAutomaticApplyOPS\">1");
    CheckResult {
        id: "nv-autoopt-off".into(),
        ok: true,
        attention: on,
        text: if on {
            "NVIDIA App automatic optimize is on. Turn it off in NVIDIA App.".into()
        } else {
            "NVIDIA App automatic optimize is off or not set.".into()
        },
    }
}

fn cmd(bin: &str, args: &[&str]) -> String {
    std::process::Command::new(bin)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcredist_result_has_id() {
        let r = vcredist();
        assert_eq!(r.id, "vcredist-check");
    }
}
