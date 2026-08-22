#[cfg(any(test, not(windows)))]
use crate::error::Error;
use crate::error::Result;
use crate::types::{Hive, RegVal};
#[cfg(any(test, not(windows)))]
use std::collections::HashMap;
#[cfg(any(test, not(windows)))]
use std::sync::Mutex;

pub const ULTIMATE_TEMPLATE: &str = "e9a42b02-3cd1-4ed4-8e39-3f6b770b171b";
pub const BALANCED_SCHEME: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
pub const TOOL_SCHEME_NAME: &str = "Clean Optimizer · Ultimate Performance";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegKeyRef {
    pub hive: Hive,
    pub path: String,
    pub name: String,
}

impl RegKeyRef {
    pub fn new(hive: Hive, path: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            hive,
            path: path.into(),
            name: name.into(),
        }
    }

}

pub trait Store: Send + Sync {
    fn get_reg(&self, key: &RegKeyRef) -> Result<Option<RegVal>>;
    fn set_reg(&self, key: &RegKeyRef, value: &RegVal) -> Result<()>;
    fn delete_reg(&self, key: &RegKeyRef) -> Result<()>;
    fn active_scheme(&self) -> Result<Option<String>>;
    fn set_active_scheme(&self, guid: &str) -> Result<()>;
    fn tool_scheme(&self) -> Result<Option<String>>;
    fn ensure_tool_scheme(&self) -> Result<String>;
    fn power_setting(&self, sub: &str, setting: &str) -> Result<Option<u32>>;
    fn set_power_setting(&self, sub: &str, setting: &str, value: u32) -> Result<()>;
    fn mmagent(&self, feature: &str) -> Result<Option<bool>>;
    fn set_mmagent(&self, feature: &str, enabled: bool) -> Result<()>;
    fn hibernate(&self) -> Result<Option<bool>>;
    fn set_hibernate(&self, on: bool) -> Result<()>;
    fn bcd(&self, name: &str) -> Result<Option<String>>;
    fn set_bcd(&self, name: &str, value: &str) -> Result<()>;
    fn delete_bcd(&self, name: &str) -> Result<()>;
    fn lock_task(&self) -> Result<bool>;
    fn set_lock_task(&self, on: bool) -> Result<()>;
    fn kv_string(&self, key: &RegKeyRef) -> Result<Option<String>> {
        match self.get_reg(key)? {
            Some(RegVal::Sz { value }) => Ok(Some(value)),
            _ => Ok(None),
        }
    }
}

#[cfg(any(test, not(windows)))]
#[derive(Default)]
struct MemoryInner {
    regs: HashMap<(Hive, String, String), RegVal>,
    scheme: Option<String>,
    tool_scheme: Option<String>,
    power: HashMap<(String, String), u32>,
    mmagent: HashMap<String, bool>,
    hibernate: Option<bool>,
    bcd: HashMap<String, String>,
    lock_task: bool,
}

#[cfg(any(test, not(windows)))]
pub struct MemoryStore {
    inner: Mutex<MemoryInner>,
}

#[cfg(any(test, not(windows)))]
impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MemoryInner::default()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, MemoryInner>> {
        self.inner
            .lock()
            .map_err(|_| Error::Msg("memory store lock poisoned".into()))
    }

    fn norm(key: &RegKeyRef) -> RegKeyRef {
        RegKeyRef {
            hive: key.hive,
            path: key.path.to_ascii_lowercase(),
            name: key.name.to_ascii_lowercase(),
        }
    }
}

#[cfg(any(test, not(windows)))]
impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, not(windows)))]
impl Store for MemoryStore {
    fn get_reg(&self, key: &RegKeyRef) -> Result<Option<RegVal>> {
        let n = Self::norm(key);
        Ok(self.lock()?.regs.get(&(n.hive, n.path, n.name)).cloned())
    }

    fn set_reg(&self, key: &RegKeyRef, value: &RegVal) -> Result<()> {
        let n = Self::norm(key);
        self.lock()?
            .regs
            .insert((n.hive, n.path, n.name), value.clone());
        Ok(())
    }

    fn delete_reg(&self, key: &RegKeyRef) -> Result<()> {
        let n = Self::norm(key);
        self.lock()?.regs.remove(&(n.hive, n.path, n.name));
        Ok(())
    }

    fn active_scheme(&self) -> Result<Option<String>> {
        Ok(self.lock()?.scheme.clone())
    }

    fn set_active_scheme(&self, guid: &str) -> Result<()> {
        self.lock()?.scheme = Some(guid.to_string());
        Ok(())
    }

    fn tool_scheme(&self) -> Result<Option<String>> {
        Ok(self.lock()?.tool_scheme.clone())
    }

    fn ensure_tool_scheme(&self) -> Result<String> {
        let mut g = self.lock()?;
        if let Some(id) = &g.tool_scheme {
            return Ok(id.clone());
        }
        let id = "11111111-2222-3333-4444-555555555555".to_string();
        g.tool_scheme = Some(id.clone());
        Ok(id)
    }

    fn power_setting(&self, sub: &str, setting: &str) -> Result<Option<u32>> {
        Ok(self
            .lock()?
            .power
            .get(&(sub.to_ascii_lowercase(), setting.to_ascii_lowercase()))
            .copied())
    }

    fn set_power_setting(&self, sub: &str, setting: &str, value: u32) -> Result<()> {
        self.lock()?.power.insert(
            (sub.to_ascii_lowercase(), setting.to_ascii_lowercase()),
            value,
        );
        Ok(())
    }

    fn mmagent(&self, feature: &str) -> Result<Option<bool>> {
        Ok(self.lock()?.mmagent.get(&feature.to_ascii_lowercase()).copied())
    }

    fn set_mmagent(&self, feature: &str, enabled: bool) -> Result<()> {
        self.lock()?
            .mmagent
            .insert(feature.to_ascii_lowercase(), enabled);
        Ok(())
    }

    fn hibernate(&self) -> Result<Option<bool>> {
        Ok(self.lock()?.hibernate)
    }

    fn set_hibernate(&self, on: bool) -> Result<()> {
        self.lock()?.hibernate = Some(on);
        Ok(())
    }

    fn bcd(&self, name: &str) -> Result<Option<String>> {
        Ok(self.lock()?.bcd.get(&name.to_ascii_lowercase()).cloned())
    }

    fn set_bcd(&self, name: &str, value: &str) -> Result<()> {
        self.lock()?
            .bcd
            .insert(name.to_ascii_lowercase(), value.to_string());
        Ok(())
    }

    fn delete_bcd(&self, name: &str) -> Result<()> {
        self.lock()?.bcd.remove(&name.to_ascii_lowercase());
        Ok(())
    }

    fn lock_task(&self) -> Result<bool> {
        Ok(self.lock()?.lock_task)
    }

    fn set_lock_task(&self, on: bool) -> Result<()> {
        self.lock()?.lock_task = on;
        Ok(())
    }
}

pub fn set_kv_item(raw: &str, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = raw
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();
    let prefix = format!("{key}=");
    let mut found = false;
    for part in &mut parts {
        if part.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) {
            *part = format!("{key}={value}");
            found = true;
            break;
        }
    }
    if !found {
        parts.push(format!("{key}={value}"));
    }
    let mut out = parts.join(";");
    if !out.is_empty() && !out.ends_with(';') {
        out.push(';');
    }
    out
}

pub fn get_kv_item(raw: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for part in raw.split(';') {
        let part = part.trim();
        if part.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()) {
            return Some(part[prefix.len()..].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_roundtrip_preserves_siblings() {
        let raw = "SwapEffectUpgradeEnable=1;AutoHDREnable=1;";
        let next = set_kv_item(raw, "SwapEffectUpgradeEnable", "0");
        assert_eq!(get_kv_item(&next, "SwapEffectUpgradeEnable").as_deref(), Some("0"));
        assert_eq!(get_kv_item(&next, "AutoHDREnable").as_deref(), Some("1"));
    }

    #[test]
    fn memory_store_reg() {
        let s = MemoryStore::new();
        let k = RegKeyRef::new(Hive::Hkcu, r"Software\Test", "Flag");
        assert!(s.get_reg(&k).unwrap().is_none());
        s.set_reg(&k, &RegVal::Dword { value: 2 }).unwrap();
        assert_eq!(s.get_reg(&k).unwrap(), Some(RegVal::Dword { value: 2 }));
        s.delete_reg(&k).unwrap();
        assert!(s.get_reg(&k).unwrap().is_none());
    }
}
