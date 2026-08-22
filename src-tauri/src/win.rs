#![cfg(windows)]

use crate::error::{Error, Result};
use crate::store::{
    brace_guid, guid_eq, normalize_guid, RegKeyRef, Store, HIGH_PERFORMANCE, TOOL_SCHEME_NAME,
    ULTIMATE_TEMPLATE,
};
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

pub fn hidden_command(bin: &str) -> Command {
    let mut command = Command::new(bin);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

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

pub fn kill_other_instances() {
    let me = std::process::id();
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(name) = exe.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let _ = hidden_command("taskkill")
        .args(["/F", "/IM", name, "/FI", &format!("PID ne {me}")])
        .status();
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
        std::process::exit(0);
    }
    Err(Error::Msg("elevation request was declined".into()))
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
    let out = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.is_empty() {
            Err(Error::Msg(format!("{cmd} failed with {}", out.status)))
        } else {
            Err(Error::Msg(format!("{cmd} failed: {err}")))
        }
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
        run_ok("powercfg", &["/setactive", &brace_guid(guid)])
    }

    fn tool_scheme(&self) -> Result<Option<String>> {
        let text = run_capture("powercfg", &["/l"])?;
        for line in text.lines() {
            if line.contains(TOOL_SCHEME_NAME) {
                if let Some(id) = extract_guid(line) {
                    persist_scheme(&id);
                    return Ok(Some(id));
                }
            }
        }
        if let Some(id) = load_persisted_scheme() {
            if scheme_listed(&text, &id) {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    fn ensure_tool_scheme(&self) -> Result<String> {
        if let Some(id) = self.tool_scheme()? {
            persist_scheme(&id);
            return Ok(id);
        }
        let list = run_capture("powercfg", &["/l"])?;
        for source in duplicate_candidates(&list) {
            match run_capture("powercfg", &["/duplicatescheme", &normalize_guid(&source)]) {
                Ok(text) => {
                    if let Some(guid) = extract_guid(&text) {
                        let _ = run_ok("powercfg", &["/changename", &guid, TOOL_SCHEME_NAME]);
                        persist_scheme(&guid);
                        return Ok(guid);
                    }
                }
                Err(_) => continue,
            }
        }
        if let Some(existing) = find_named_scheme(&list, &["Ultimate Performance", "卓越性能"]) {
            persist_scheme(&existing);
            return Ok(existing);
        }
        Err(Error::Msg(
            "could not create or locate an Ultimate Performance power scheme".into(),
        ))
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
        let on = text.to_ascii_lowercase().contains("cleanoptimizer-powerplanlock");
        if on {
            if let Some(guid) = self.active_scheme()? {
                let _ = install_hidden_lock_task(&guid);
            }
        }
        Ok(on)
    }

    fn set_lock_task(&self, on: bool) -> Result<()> {
        if on {
            let guid = self.active_scheme()?.unwrap_or_default();
            install_hidden_lock_task(&guid)
        } else {
            let _ = run_ok("schtasks", &["/delete", "/tn", "CleanOptimizer-PowerPlanLock", "/f"]);
            if let Ok(path) = lock_script_path() {
                let _ = std::fs::remove_file(path);
            }
            Ok(())
        }
    }
}

fn lock_script_path() -> Result<PathBuf> {
    Ok(crate::paths::Paths::live()?.root.join("power-lock.vbs"))
}

pub(crate) fn lock_script_text(guid: &str) -> String {
    format!(
        "Set sh = CreateObject(\"WScript.Shell\")\r\nsh.Run \"powercfg.exe /setactive {}\", 0, False\r\n",
        brace_guid(guid)
    )
}

fn install_hidden_lock_task(guid: &str) -> Result<()> {
    let path = lock_script_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, lock_script_text(guid))?;
    let tr = format!("wscript.exe //B //Nologo \"{}\"", path.display());
    run_ok(
        "schtasks",
        &[
            "/create",
            "/tn",
            "CleanOptimizer-PowerPlanLock",
            "/tr",
            &tr,
            "/sc",
            "minute",
            "/mo",
            "1",
            "/f",
        ],
    )
}

pub(crate) fn extract_guid(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        if let Some(rel_end) = text[start..].find('}') {
            let inner = &text[start + 1..start + rel_end];
            if looks_like_guid(inner) {
                return Some(brace_guid(inner));
            }
        }
    }
    find_bare_guid(text).map(|g| brace_guid(&g))
}

fn looks_like_guid(raw: &str) -> bool {
    let b = raw.as_bytes();
    b.len() == 36
        && b[8] == b'-'
        && b[13] == b'-'
        && b[18] == b'-'
        && b[23] == b'-'
        && raw.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn find_bare_guid(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.len() < 36 {
        return None;
    }
    for i in 0..=bytes.len() - 36 {
        let slice = &text[i..i + 36];
        if looks_like_guid(slice) {
            return Some(slice.to_string());
        }
    }
    None
}

pub(crate) fn duplicate_candidates(list_text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |raw: &str| {
        let guid = brace_guid(raw);
        if !out.iter().any(|x| guid_eq(x, &guid)) {
            out.push(guid);
        }
    };
    push(ULTIMATE_TEMPLATE);
    if let Some(found) = find_named_scheme(list_text, &["Ultimate Performance", "卓越性能"]) {
        push(&found);
    }
    push(HIGH_PERFORMANCE);
    out
}

fn find_named_scheme(list_text: &str, needles: &[&str]) -> Option<String> {
    for line in list_text.lines() {
        if line.contains(TOOL_SCHEME_NAME) {
            continue;
        }
        if needles.iter().any(|n| line.contains(*n)) {
            if let Some(guid) = extract_guid(line) {
                return Some(guid);
            }
        }
    }
    None
}

fn scheme_listed(list_text: &str, guid: &str) -> bool {
    list_text.lines().any(|line| {
        extract_guid(line)
            .map(|g| guid_eq(&g, guid))
            .unwrap_or(false)
    })
}

fn persist_scheme(guid: &str) {
    let Ok(paths) = crate::paths::Paths::live() else {
        return;
    };
    let path = paths.user.join("config").join("power-scheme.json");
    let body = serde_json::json!({ "guid": brace_guid(guid) });
    let _ = std::fs::write(path, body.to_string());
}

fn load_persisted_scheme() -> Option<String> {
    let paths = crate::paths::Paths::live().ok()?;
    let path = paths.user.join("config").join("power-scheme.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let guid = v.get("guid")?.as_str()?;
    if looks_like_guid(&normalize_guid(guid)) {
        Some(brace_guid(guid))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_guid_reads_braces_and_bare() {
        assert_eq!(
            extract_guid("Power Scheme GUID: {381b4222-f694-41f0-9685-ff5bb260df2e}  (Balanced)"),
            Some("{381b4222-f694-41f0-9685-ff5bb260df2e}".into())
        );
        assert_eq!(
            extract_guid("Power Scheme GUID: 7b815e17-f3fd-4f2b-a0f9-52b2dd2c1484  (custom)"),
            Some("{7b815e17-f3fd-4f2b-a0f9-52b2dd2c1484}".into())
        );
    }

    #[test]
    fn ensure_tool_scheme_resolves() {
        let list = "\
Existing Power Schemes (* Active)
Power Scheme GUID: 0d8ec965-6eba-492e-ac73-cffba22c7c4c  (Ultimate Performance)
Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)
";
        let cands = duplicate_candidates(list);
        assert!(cands.iter().any(|g| guid_eq(g, ULTIMATE_TEMPLATE)));
        assert!(cands.iter().any(|g| guid_eq(g, "0d8ec965-6eba-492e-ac73-cffba22c7c4c")));
        assert!(cands.iter().any(|g| guid_eq(g, HIGH_PERFORMANCE)));
    }

    #[test]
    fn live_active_scheme_is_some() {
        let text = run_capture("powercfg", &["/getactivescheme"]).unwrap();
        assert!(
            extract_guid(&text).is_some(),
            "active scheme text had no GUID: {text}"
        );
    }

    #[test]
    fn lock_script_runs_powercfg_hidden() {
        let text = lock_script_text("7b815e17-f3fd-4f2b-a0f9-52b2dd2c1484");
        assert!(text.contains("powercfg.exe /setactive {7b815e17-f3fd-4f2b-a0f9-52b2dd2c1484}"));
        assert!(text.contains(", 0, False"));
    }
}
