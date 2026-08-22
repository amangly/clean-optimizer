use crate::error::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn append(user: &Path, line: &str) -> Result<()> {
    let dir = user.join("logs");
    fs::create_dir_all(&dir)?;
    let path = dir.join("run.log");
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}  {line}", crate::backup::now_secs())?;
    Ok(())
}

pub fn read_tail(user: &Path, max_bytes: usize) -> Result<String> {
    let path = user.join("logs").join("run.log");
    if !path.exists() {
        return Ok(String::new());
    }
    let data = fs::read(path)?;
    if data.len() <= max_bytes {
        return Ok(String::from_utf8_lossy(&data).to_string());
    }
    Ok(String::from_utf8_lossy(&data[data.len() - max_bytes..]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn append_and_read() {
        let tmp = tempdir().unwrap();
        append(tmp.path(), "applied game-mode").unwrap();
        let text = read_tail(tmp.path(), 4096).unwrap();
        assert!(text.contains("applied game-mode"));
    }
}
