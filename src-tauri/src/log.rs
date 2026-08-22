use crate::error::Result;
use crate::types::ItemResult;
use std::fs;
use std::io::Write;
use std::path::Path;

fn oneline(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn result_status(r: &ItemResult) -> &'static str {
    if !r.ok {
        "fail"
    } else if r.skipped {
        "skip"
    } else {
        "ok"
    }
}

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

pub fn append_run(
    user: &Path,
    verb: &str,
    header: &str,
    results: &[ItemResult],
    footer: &str,
) -> Result<()> {
    append(user, header)?;
    for r in results {
        let mut flags = Vec::new();
        if r.changed {
            flags.push("changed");
        }
        if r.attention {
            flags.push("attention");
        }
        if r.reboot {
            flags.push("reboot");
        }
        let flag = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(","))
        };
        append(
            user,
            &format!(
                "{verb} {} {} {}{} {}",
                r.id,
                result_status(r),
                r.name,
                flag,
                oneline(&r.message)
            ),
        )?;
    }
    append(user, footer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ItemResult;
    use tempfile::tempdir;

    #[test]
    fn append_and_read() {
        let tmp = tempdir().unwrap();
        append(tmp.path(), "applied game-mode").unwrap();
        let text = read_tail(tmp.path(), 4096).unwrap();
        assert!(text.contains("applied game-mode"));
    }

    #[test]
    fn append_run_writes_each_item() {
        let tmp = tempdir().unwrap();
        let item = ItemResult {
            id: "game-mode".into(),
            name: "Game Mode".into(),
            ok: true,
            changed: true,
            skipped: false,
            attention: false,
            reboot: false,
            message: "wrote 2\nchange(s)".into(),
        };
        append_run(
            tmp.path(),
            "apply",
            "apply start n=1",
            &[item],
            "apply done ok=1 fail=0 skip=0",
        )
        .unwrap();
        let text = read_tail(tmp.path(), 4096).unwrap();
        assert!(text.contains("apply start n=1"));
        assert!(text.contains("apply game-mode ok Game Mode [changed] wrote 2 change(s)"));
        assert!(text.contains("apply done ok=1 fail=0 skip=0"));
    }
}
