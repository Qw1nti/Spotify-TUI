use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
};

use crate::dotenv::Dotenv;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_hosts: Vec<String>,
    pub redirect_ports: Vec<u16>,
    pub theme: ThemeConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub accent: String,
    pub accent_soft: String,
    pub background: String,
    pub surface: String,
    pub text: String,
    pub muted: String,
    pub danger: String,
    pub success: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub list_page_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_hosts: vec!["127.0.0.1".into()],
            redirect_ports: vec![8890],
            theme: ThemeConfig {
                accent: "Cyan".into(),
                accent_soft: "LightCyan".into(),
                background: "Black".into(),
                surface: "DarkGray".into(),
                text: "White".into(),
                muted: "Gray".into(),
                danger: "LightRed".into(),
                success: "LightGreen".into(),
            },
            ui: UiConfig { list_page_size: 20 },
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: "Cyan".into(),
            accent_soft: "LightCyan".into(),
            background: "Black".into(),
            surface: "DarkGray".into(),
            text: "White".into(),
            muted: "Gray".into(),
            danger: "LightRed".into(),
            success: "LightGreen".into(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { list_page_size: 20 }
    }
}

impl Config {
    pub fn load(env: &Dotenv) -> Result<Self> {
        let path = Self::path();
        let mut cfg = if let Ok(raw) = fs::read_to_string(&path) {
            let mut cfg: Config = serde_yaml::from_str(&raw)
                .with_context(|| format!("failed to parse config {}", path.display()))?;
            if cfg.redirect_hosts.is_empty() {
                cfg.redirect_hosts = vec!["127.0.0.1".into()];
            }
            if cfg.redirect_ports.is_empty() {
                cfg.redirect_ports = vec![8890];
            }
            if cfg.ui.list_page_size == 0 {
                cfg.ui.list_page_size = 20;
            }
            cfg
        } else {
            Self::default()
        };

        if let Some(client_id) = env.get("SPOTIFY_CLIENT_ID") {
            if !client_id.trim().is_empty() {
                cfg.client_id = client_id.to_string();
            }
        }

        if let Some(client_secret) = env.get("SPOTIFY_CLIENT_SECRET") {
            if !client_secret.trim().is_empty() {
                cfg.client_secret = client_secret.to_string();
            }
        }

        Ok(cfg)
    }

    pub(crate) fn path() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config/spotifytui/config.yml")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_yaml::to_string(self)?)?;
        Ok(())
    }
}
