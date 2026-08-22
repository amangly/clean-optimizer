#![cfg(windows)]

use crate::error::{Error, Result};
use crate::store::{RegKeyRef, Store, TOOL_SCHEME_NAME, ULTIMATE_TEMPLATE};
use crate::types::{Hive, RegVal};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_BINARY};
use winreg::{RegKey, RegValue};
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_admin() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == FALSE {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != FALSE && elevation.TokenIsElevated != 0
    }
}

pub fn current_user_sid() -> Result<String> {
    let output = run_capture(
        "whoami",
        &["/user", "/fo", "csv", "/nh"],
    )?;
    let line = output.lines().next().unwrap_or("");
    let sid = line
        .split(',')
        .last()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .to_string();
    if sid.starts_with("S-1-") {
        Ok(sid)
    } else {
        Err(Error::Msg("could not read user SID".into()))
    }
}

pub fn process_path(name: &str) -> Option<String> {
    let output = run_capture("powershell", &[
        "-NoProfile",
        "-Command",
        &format!(
            "(Get-Process -Name '{name}' -ErrorAction SilentlyContinue | Select-Object -First 1).Path"
        ),
    ])
    .ok()?;
    let path = output.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

pub fn uninstall_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (hive, path) in [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ] {
        let Ok(key) = RegKey::predef(hive).open_subkey(path) else {
            continue;
        };
        for name in key.enum_keys().flatten() {
            let Ok(sub) = key.open_subkey(&name) else { continue };
            let display: String = sub.get_value("DisplayName").unwrap_or_default();
            if !looks_like_game(&display) {
                continue;
            }
            for field in ["InstallLocation", "DisplayIcon", "UninstallString"] {
                let raw: String = sub.get_value(field).unwrap_or_default();
                if let Some(root) = parent_if_exists(&raw) {
                    roots.push(root);
                }
            }
        }
    }
    roots
}

pub fn platform_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Tencent\WeGame") {
        if let Ok(p) = hkcu.get_value::<String, _>("InstallPath") {
            let path = PathBuf::from(p);
            if path.exists() {
                roots.push(path);
            }
        }
    }
    if let Ok(steam) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam") {
        if let Ok(p) = steam.get_value::<String, _>("SteamPath") {
            let steam = PathBuf::from(p);
            let vdf = steam.join(r"steamapps\libraryfolders.vdf");
            if let Ok(text) = std::fs::read_to_string(&vdf) {
                for cap in text.split("\"path\"").skip(1) {
                    if let Some(path) = cap.split('"').nth(1) {
                        let lib = PathBuf::from(path.replace("\\\\", "\\"));
                        let game = lib.join(r"steamapps\common\Delta Force");
                        if game.exists() {
                            roots.push(game);
                        }
                    }
                }
            }
        }
    }
    roots
}

pub fn relaunch_elevated() -> Result<()> {
    let exe = std::env::current_exe()?;
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process -FilePath '{}' -Verb RunAs",
                exe.display().to_string().replace('\'', "''")
            ),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Msg("elevation request was declined".into()))
    }
}

fn looks_like_game(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("delta force") || n.contains("deltaforce") || name.contains("三角洲")
}

fn parent_if_exists(raw: &str) -> Option<PathBuf> {
    let cleaned = raw.trim().trim_matches('"');
    if cleaned.is_empty() {
        return None;
    }
    let path = PathBuf::from(cleaned.split_whitespace().next().unwrap_or(cleaned));
    if path.is_file() {
        path.parent().map(PathBuf::from)
    } else if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn run_capture(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_ok(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Msg(format!("{cmd} failed with {status}")))
    }
}

fn hive_key(hive: Hive) -> RegKey {
    match hive {
        Hive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
        Hive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
    }
}

pub struct WindowsStore;

impl Store for WindowsStore {
    fn get_reg(&self, key: &RegKeyRef) -> Result<Option<RegVal>> {
        let hive = hive_key(key.hive);
        let Ok(sub) = hive.open_subkey_with_flags(&key.path, KEY_READ) else {
            return Ok(None);
        };
        if let Ok(v) = sub.get_value::<u32, _>(&key.name) {
            return Ok(Some(RegVal::Dword { value: v }));
        }
        if let Ok(v) = sub.get_value::<String, _>(&key.name) {
            return Ok(Some(RegVal::Sz { value: v }));
        }
        if let Ok(raw) = sub.get_raw_value(&key.name) {
            if raw.vtype == REG_BINARY {
                return Ok(Some(RegVal::Bin { value: raw.bytes }));
            }
        }
        Ok(None)
    }

    fn set_reg(&self, key: &RegKeyRef, value: &RegVal) -> Result<()> {
        let hive = hive_key(key.hive);
        let (sub, _) = hive.create_subkey_with_flags(&key.path, KEY_SET_VALUE)?;
        match value {
            RegVal::Dword { value } => sub.set_value(&key.name, value)?,
            RegVal::Sz { value } => sub.set_value(&key.name, value)?,
            RegVal::Bin { value } => sub.set_raw_value(
                &key.name,
                &RegValue {
                    bytes: value.clone(),
                    vtype: REG_BINARY,
                },
            )?,
        }
        Ok(())
    }

    fn delete_reg(&self, key: &RegKeyRef) -> Result<()> {
        let hive = hive_key(key.hive);
        let Ok(sub) = hive.open_subkey_with_flags(&key.path, KEY_SET_VALUE) else {
            return Ok(());
        };
        let _ = sub.delete_value(&key.name);
        Ok(())
    }

    fn active_scheme(&self) -> Result<Option<String>> {
        let text = run_capture("powercfg", &["/getactivescheme"])?;
        Ok(extract_guid(&text))
    }

    fn set_active_scheme(&self, guid: &str) -> Result<()> {
        run_ok("powercfg", &["/setactive", guid])
    }

    fn tool_scheme(&self) -> Result<Option<String>> {
        let text = run_capture("powercfg", &["/l"])?;
        for line in text.lines() {
            if line.contains(TOOL_SCHEME_NAME) {
                return Ok(extract_guid(line));
            }
        }
        Ok(None)
    }

    fn ensure_tool_scheme(&self) -> Result<String> {
        if let Some(id) = self.tool_scheme()? {
            return Ok(id);
        }
        let text = run_capture("powercfg", &["/duplicatescheme", ULTIMATE_TEMPLATE])?;
        let guid = extract_guid(&text).ok_or_else(|| Error::Msg("powercfg did not return a scheme GUID".into()))?;
        run_ok("powercfg", &["/changename", &guid, TOOL_SCHEME_NAME])?;
        Ok(guid)
    }

    fn power_setting(&self, sub: &str, setting: &str) -> Result<Option<u32>> {
        let scheme = self.active_scheme()?.unwrap_or_default();
        if scheme.is_empty() {
            return Ok(None);
        }
        let text = run_capture("powercfg", &["/q", &scheme, sub, setting])?;
        for line in text.lines() {
            if line.contains("Current AC Power Setting Index") {
                if let Some(hex) = line.split(':').last() {
                    let hex = hex.trim().trim_start_matches("0x");
                    if let Ok(v) = u32::from_str_radix(hex, 16) {
                        return Ok(Some(v));
                    }
                }
            }
        }
        Ok(None)
    }

    fn set_power_setting(&self, sub: &str, setting: &str, value: u32) -> Result<()> {
        let scheme = self.active_scheme()?.unwrap_or_default();
        if scheme.is_empty() {
            return Err(Error::Msg("no active power scheme".into()));
        }
        let _ = run_ok("powercfg", &["-attributes", sub, setting, "-ATTRIB_HIDE"]);
        run_ok(
            "powercfg",
            &["/setacvalueindex", &scheme, sub, setting, &value.to_string()],
        )?;
        self.set_active_scheme(&scheme)
    }

    fn mmagent(&self, feature: &str) -> Result<Option<bool>> {
        let name = match feature {
            "mc" => "MemoryCompression",
            "pc" => "PageCombining",
            _ => return Ok(None),
        };
        let text = run_capture(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                &format!("(Get-MMAgent).{name}"),
            ],
        )?;
        match text.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => Ok(None),
        }
    }

    fn set_mmagent(&self, feature: &str, enabled: bool) -> Result<()> {
        let cmd = match (feature, enabled) {
            ("mc", false) => "Disable-MMAgent -MemoryCompression",
            ("mc", true) => "Enable-MMAgent -MemoryCompression",
            ("pc", false) => "Disable-MMAgent -PageCombining",
            ("pc", true) => "Enable-MMAgent -PageCombining",
            _ => return Err(Error::Msg("unknown mmagent feature".into())),
        };
        run_ok("powershell", &["-NoProfile", "-Command", cmd])
    }

    fn hibernate(&self) -> Result<Option<bool>> {
        let path = r"C:\hiberfil.sys";
        Ok(Some(std::path::Path::new(path).exists()))
    }

    fn set_hibernate(&self, on: bool) -> Result<()> {
        run_ok("powercfg", &["-h", if on { "on" } else { "off" }])
    }

    fn bcd(&self, name: &str) -> Result<Option<String>> {
        let text = run_capture("bcdedit", &["/enum", "{current}"])?;
        for line in text.lines() {
            if line.to_ascii_lowercase().starts_with(&name.to_ascii_lowercase()) {
                return Ok(line.split_whitespace().last().map(|s| s.to_string()));
            }
        }
        Ok(None)
    }

    fn set_bcd(&self, name: &str, value: &str) -> Result<()> {
        run_ok("bcdedit", &["/set", name, value])
    }

    fn delete_bcd(&self, name: &str) -> Result<()> {
        run_ok("bcdedit", &["/deletevalue", name])
    }

    fn lock_task(&self) -> Result<bool> {
        let text = run_capture("schtasks", &["/query", "/tn", "CleanOptimizer-PowerPlanLock"])?;
        Ok(text.to_ascii_lowercase().contains("cleanoptimizer-powerplanlock"))
    }

    fn set_lock_task(&self, on: bool) -> Result<()> {
        if on {
            let guid = self.active_scheme()?.unwrap_or_default();
            run_ok(
                "schtasks",
                &[
                    "/create",
                    "/tn",
                    "CleanOptimizer-PowerPlanLock",
                    "/tr",
                    &format!("powercfg /setactive {guid}"),
                    "/sc",
                    "minute",
                    "/mo",
                    "1",
                    "/f",
                ],
            )
        } else {
            let _ = run_ok("schtasks", &["/delete", "/tn", "CleanOptimizer-PowerPlanLock", "/f"]);
            Ok(())
        }
    }
}

fn extract_guid(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text[start..].find('}')?;
    Some(text[start..=start + end].to_string())
}
