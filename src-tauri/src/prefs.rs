use crate::error::Result;
use crate::types::Prefs;
use std::fs;
use std::path::Path;

pub fn load(user: &Path) -> Result<Prefs> {
    let path = user.join("config").join("prefs.json");
    if !path.exists() {
        return Ok(Prefs::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn save(user: &Path, prefs: &Prefs) -> Result<()> {
    let dir = user.join("config");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("prefs.json"), serde_json::to_vec_pretty(prefs)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn disclaimer_persists() {
        let tmp = tempdir().unwrap();
        let mut p = Prefs::default();
        p.disclaimer_accepted = true;
        p.telemetry = false;
        save(tmp.path(), &p).unwrap();
        let loaded = load(tmp.path()).unwrap();
        assert!(loaded.disclaimer_accepted);
        assert!(!loaded.telemetry);
    }
}
