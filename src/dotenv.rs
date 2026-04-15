use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Default)]
pub struct Dotenv {
    values: HashMap<String, String>,
}

impl Dotenv {
    pub fn load() -> Result<Self> {
        let mut values = HashMap::new();
        let path = current_dir_env_path()?;
        if let Ok(raw) = fs::read_to_string(&path) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    values.insert(key.trim().to_string(), strip_quotes(value.trim()));
                }
            }
        }
        Ok(Self { values })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }
}

fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        stripped.to_string()
    } else if let Some(stripped) = trimmed.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        stripped.to_string()
    } else {
        trimmed.to_string()
    }
}

fn current_dir_env_path() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(".env"))
}
