use crate::error::{Error, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const SCHEMA: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupOp {
    pub item_id: String,
    pub kind: String,
    pub target: String,
    pub original: serde_json::Value,
    pub written: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupDoc {
    pub schema: u32,
    pub apply_id: String,
    pub created: u64,
    pub items: Vec<String>,
    pub ops: Vec<BackupOp>,
    pub hmac: String,
}

#[derive(Serialize)]
struct SignBody<'a> {
    schema: u32,
    apply_id: &'a str,
    created: u64,
    items: &'a [String],
    ops: &'a [BackupOp],
}

pub struct BackupStore {
    pub dir: PathBuf,
    key: Vec<u8>,
}

impl BackupStore {
    pub fn open(root: &Path) -> Result<Self> {
        let dir = root.join("backup");
        fs::create_dir_all(&dir)?;
        let key_path = root.join("backup.key");
        let key = if key_path.exists() {
            fs::read(&key_path)?
        } else {
            let bytes = Uuid::new_v4().as_bytes().to_vec();
            fs::write(&key_path, &bytes)?;
            bytes
        };
        Ok(Self { dir, key })
    }

    pub fn sign(&self, apply_id: &str, created: u64, items: &[String], ops: &[BackupOp]) -> String {
        let body = SignBody {
            schema: SCHEMA,
            apply_id,
            created,
            items,
            ops,
        };
        let bytes = serde_json::to_vec(&body).expect("sign body");
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("hmac key");
        mac.update(&bytes);
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify(&self, doc: &BackupDoc) -> Result<()> {
        let expected = self.sign(&doc.apply_id, doc.created, &doc.items, &doc.ops);
        if expected != doc.hmac {
            return Err(Error::BackupTampered);
        }
        Ok(())
    }

    pub fn write(&self, items: &[String], ops: Vec<BackupOp>) -> Result<(String, PathBuf)> {
        let apply_id = Uuid::new_v4().to_string();
        let created = now_secs();
        let hmac = self.sign(&apply_id, created, items, &ops);
        let doc = BackupDoc {
            schema: SCHEMA,
            apply_id: apply_id.clone(),
            created,
            items: items.to_vec(),
            ops,
            hmac,
        };
        let pending = self.dir.join(format!("backup-{apply_id}.pending.json"));
        let final_path = self.dir.join(format!("backup-{apply_id}.json"));
        let bytes = serde_json::to_vec_pretty(&doc)?;
        fs::write(&pending, &bytes)?;
        fs::rename(&pending, &final_path)?;
        Ok((apply_id, final_path))
    }

    pub fn list_active(&self) -> Result<Vec<BackupDoc>> {
        let mut docs = Vec::new();
        if !self.dir.exists() {
            return Ok(docs);
        }
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.starts_with("backup-") || !name.ends_with(".json") || name.contains(".restored") {
                continue;
            }
            if name.contains(".pending") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            let doc: BackupDoc = serde_json::from_str(&text)?;
            self.verify(&doc)?;
            docs.push(doc);
        }
        docs.sort_by_key(|d| d.created);
        Ok(docs)
    }

    pub fn write_receipt(&self, apply_id: &str, item_ids: &[String]) -> Result<PathBuf> {
        let path = self.dir.join(format!("restore-receipt-{}.json", Uuid::new_v4()));
        let body = serde_json::json!({
            "schema": 1,
            "applyId": apply_id,
            "items": item_ids,
            "created": now_secs(),
        });
        fs::write(&path, serde_json::to_vec_pretty(&body)?)?;
        Ok(path)
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hmac_roundtrip() {
        let tmp = tempdir().unwrap();
        let store = BackupStore::open(tmp.path()).unwrap();
        let op = BackupOp {
            item_id: "game-mode".into(),
            kind: "reg".into(),
            target: "hkcu\\software\\microsoft\\gamebar\\autogamemodeenabled".into(),
            original: serde_json::Value::Null,
            written: serde_json::json!({"kind":"dword","value":1}),
        };
        let (id, path) = store.write(&["game-mode".into()], vec![op]).unwrap();
        assert!(path.exists());
        let docs = store.list_active().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].apply_id, id);
    }

    #[test]
    fn tamper_is_rejected() {
        let tmp = tempdir().unwrap();
        let store = BackupStore::open(tmp.path()).unwrap();
        let (_, path) = store.write(&["dvr-off".into()], vec![]).unwrap();
        let mut doc: BackupDoc = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        doc.items.push("hags".into());
        assert!(store.verify(&doc).is_err());
    }
}
