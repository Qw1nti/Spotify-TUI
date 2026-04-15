use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

pub fn print_help() {
    println!(
        "\
spotifytui - Spotify terminal client

Usage:
  spotifytui                Run the TUI
  spotifytui help           Show this help
  spotifytui onboard        Run first-time setup wizard
  spotifytui uninstall      Remove config, cache, data, and the installed binary if possible
  spotifytui --help         Show this help

Commands:
  onboard   Re-run the interactive setup wizard for client id / secret
  uninstall Remove local app data and try to remove the installed command

Environment:
  SPOTIFY_CLIENT_ID        Override client id during setup
  SPOTIFY_CLIENT_SECRET    Override secret during setup
  SPOTIFY_CALLBACK_URL     Paste full callback URL to skip browser prompt
"
    );
}

pub fn run_onboard() -> Result<()> {
    let env = crate::dotenv::Dotenv::load()?;
    let mut config = crate::config::Config::load(&env)?;
    config = crate::setup::run_wizard(config, &env)?;
    config.save().context("failed to write config")?;
    println!("Saved config to {}", crate::config::Config::path().display());
    Ok(())
}

pub fn run_uninstall() -> Result<()> {
    println!("Removing app data...");
    remove_dir(crate::config_dir("spotifytui"));
    remove_dir(crate::home_dir(".cache/spotifytui"));
    remove_dir(crate::home_dir(".local/share/spotifytui"));
    remove_dir(crate::home_dir(".local/share/spotifytui/logs"));
    remove_tokens()?;
    try_remove_current_exe()?;
    println!("Done.");
    Ok(())
}

fn remove_tokens() -> Result<()> {
    let path = crate::home_dir(".config/spotifytui/tokens.yml");
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    let config = crate::home_dir(".config/spotifytui/config.yml");
    if config.exists() {
        fs::remove_file(&config).with_context(|| format!("failed to remove {}", config.display()))?;
    }
    let dir = crate::home_dir(".config/spotifytui");
    if dir.exists() {
        let _ = fs::remove_dir(&dir);
    }
    Ok(())
}

fn try_remove_current_exe() -> Result<()> {
    let exe = std::env::current_exe().context("failed to locate current executable")?;
    if is_removable_install(&exe) {
        fs::remove_file(&exe).with_context(|| format!("failed to remove {}", exe.display()))?;
        println!("Removed installed command: {}", exe.display());
    } else {
        println!(
            "Could not remove executable automatically: {}",
            exe.display()
        );
        println!("If needed, remove it manually or run ./uninstall.sh from the repo root.");
    }
    Ok(())
}

fn is_removable_install(path: &PathBuf) -> bool {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home {
        if path.starts_with(home.join(".local/bin")) {
            return true;
        }
    }
    if cfg!(unix) {
        path.starts_with("/usr/local/bin")
    } else {
        false
    }
}

fn remove_dir(path: PathBuf) {
    if path.exists() {
        let _ = fs::remove_dir_all(path);
    }
}
