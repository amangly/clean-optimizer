use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("administrator rights required for {0}")]
    AdminRequired(String),
    #[error("risky item {0} needs explicit confirmation")]
    Risky(String),
    #[error("game path required for {0}")]
    GamePath(String),
    #[error("backup hmac mismatch")]
    BackupTampered,
    #[error("unsupported game executable")]
    BadGamePath,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Msg(value.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Msg(value.to_string())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Msg(value)
    }
}
