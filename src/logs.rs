use anyhow::Result;
use std::{fs, io::Write, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

pub fn log_error(context: &str, err: &anyhow::Error) -> Result<()> {
    let dir = logs_dir();
    fs::create_dir_all(&dir)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("errors.log"))?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    writeln!(file, "[{ts}] {context}: {err:?}")?;
    Ok(())
}

pub fn ensure_logs_dir() -> Result<()> {
    fs::create_dir_all(logs_dir())?;
    Ok(())
}

pub fn logs_dir() -> PathBuf {
    crate::home_dir(".local/share/spotifytui/logs")
}
