use anyhow::{Context, Result};
use rpassword::read_password;
use std::io::{self, Write};

use crate::{config::Config, dotenv::Dotenv};

pub fn ensure_config(env: &Dotenv) -> Result<Config> {
    let mut config = Config::load(env)?;
    let needs_setup = !Config::path().exists() || config.client_id.trim().is_empty();

    if needs_setup {
        config = run_wizard(config, env)?;
        config.save().context("failed to write config")?;
    }

    Ok(config)
}

fn run_wizard(mut config: Config, env: &Dotenv) -> Result<Config> {
    println!("Spotify TUI setup");
    println!("Add this redirect URI to your Spotify app:");
    println!("  http://127.0.0.1:8890/callback");
    println!();
    println!("If you already added it, press Enter.");
    pause()?;

    config.client_id = prompt_value(
        "Spotify App Client ID",
        env.get("SPOTIFY_CLIENT_ID").or(Some(&config.client_id)),
    )?;
    config.client_secret = prompt_secret(
        "Spotify App Client Secret",
        env.get("SPOTIFY_CLIENT_SECRET")
            .or(Some(&config.client_secret)),
    )?;

    if config.client_id.trim().is_empty() {
        return Err(anyhow::anyhow!("client id cannot be empty"));
    }

    Ok(config)
}

fn prompt_value(label: &str, default: Option<&str>) -> Result<String> {
    let default_value = default.unwrap_or("").trim().to_string();
    if !default_value.is_empty() {
        print!("{label} [{default_value}]: ");
    } else {
        print!("{label}: ");
    }
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let value = input.trim().to_string();
    if value.is_empty() {
        Ok(default_value)
    } else {
        Ok(value)
    }
}

fn prompt_secret(label: &str, default: Option<&str>) -> Result<String> {
    let default_value = default.unwrap_or("").trim().to_string();
    if !default_value.is_empty() {
        print!("{label} [stored value hidden, press Enter to keep]: ");
        io::stdout().flush()?;
        let value = read_password()?;
        if value.trim().is_empty() {
            Ok(default_value)
        } else {
            Ok(value.trim().to_string())
        }
    } else {
        print!("{label}: ");
        io::stdout().flush()?;
        Ok(read_password()?.trim().to_string())
    }
}

fn pause() -> Result<()> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(())
}
