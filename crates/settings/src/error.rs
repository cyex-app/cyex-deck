use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Messages are shown to the user as-is, so keep them short and plain.
#[derive(Debug, Error)]
pub enum Error {
    #[error("couldn’t read or write the settings files: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings aren’t valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("that isn’t a cyex settings file")]
    NotSettings,
    #[error("these settings come from a newer version of cyex deck")]
    TooNew,
    #[error("unknown settings section \"{0}\"")]
    UnknownPartition(String),
    #[error("that share code is broken or incomplete")]
    BadCode,
    #[error("that’s too big for cyex settings")]
    TooBig,
    #[error("there’s no backup from {0}")]
    NoBackup(String),
}
