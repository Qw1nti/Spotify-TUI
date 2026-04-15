use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use url::Url;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenCache {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

pub async fn authenticate(config: &Config, callback_override: Option<String>) -> Result<AuthTokens> {
    if config.client_id.trim().is_empty() {
        return Err(anyhow!("client_id missing in config"));
    }

    if let Some(cached) = load_cached_tokens()? {
        if !is_expired(cached.expires_at) {
            return Ok(AuthTokens {
                access_token: cached.access_token,
                refresh_token: cached.refresh_token,
                expires_at: cached.expires_at,
            });
        }

        if let Ok(refreshed) =
            refresh_tokens(&config.client_id, &config.client_secret, &cached.refresh_token).await
        {
            save_cached_tokens(&refreshed)?;
            return Ok(refreshed);
        }
    }

    let use_secret = !config.client_secret.trim().is_empty();
    let verifier = if use_secret {
        None
    } else {
        Some(pkce_verifier())
    };
    let challenge = verifier.as_deref().map(pkce_challenge);
    let (listener, redirect_port) = bind_listener(&config.redirect_ports)?;
    let redirect_host = config
        .redirect_hosts
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".into());
    let redirect_uri = format!("http://{}:{}/callback", redirect_host, redirect_port);
    let state = random_state();
    let scope = [
        "user-read-playback-state",
        "user-modify-playback-state",
        "user-read-currently-playing",
        "user-library-read",
        "user-library-modify",
        "playlist-read-private",
        "playlist-read-collaborative",
        "user-read-recently-played",
        "user-top-read",
        "user-follow-read",
    ]
    .join(" ");

    let auth_url = Url::parse_with_params(
        "https://accounts.spotify.com/authorize",
        {
            let mut params = vec![
                ("client_id", config.client_id.as_str()),
                ("response_type", "code"),
                ("redirect_uri", redirect_uri.as_str()),
                ("state", state.as_str()),
                ("scope", scope.as_str()),
            ];
            if let Some(challenge) = challenge.as_deref() {
                params.push(("code_challenge_method", "S256"));
                params.push(("code_challenge", challenge));
            }
            params
        },
    )?;

    let captured = Arc::new(Mutex::new(None::<String>));
    let server_capture = Arc::clone(&captured);
    listener.set_nonblocking(true)?;

    std::thread::spawn(move || {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(240) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if let Some(url) = read_http_request(&mut stream) {
                        let _ = respond_html(&mut stream, "Login complete. You can return to Spotify TUI.");
                        *server_capture.lock().unwrap() = Some(url);
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => break,
            }
        }
    });

    let _ = webbrowser::open(auth_url.as_str());
    println!("Open this URL if browser did not launch:\n{auth_url}");
    println!("If redirect fails, paste callback URL and press Enter:");

    let callback_url = if let Some(url) = callback_override {
        url
    } else {
        wait_for_callback(captured)?
    };
    let returned = Url::parse(&callback_url)?;
    let returned_state = returned
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    if returned_state != state {
        return Err(anyhow!("spotify oauth state mismatch"));
    }
    let code = returned
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| anyhow!("authorization code missing from callback"))?;

    let tokens = exchange_code(
        &config.client_id,
        &config.client_secret,
        &redirect_uri,
        &code,
        verifier.as_deref(),
    )
    .await?;
    save_cached_tokens(&tokens)?;
    Ok(tokens)
}

fn bind_listener(ports: &[u16]) -> Result<(TcpListener, u16)> {
    for port in ports {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", *port)) {
            return Ok((listener, *port));
        }
    }
    Err(anyhow!("no available redirect port found"))
}

fn random_state() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

fn pkce_verifier() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(96)
        .map(char::from)
        .collect()
}

fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    verifier: Option<&str>,
) -> Result<AuthTokens> {
    let client = Client::new();
    let mut form = vec![
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret));
    }
    if let Some(verifier) = verifier {
        form.push(("code_verifier", verifier));
    }
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .form(&form)
        .send()
        .await?
        .error_for_status()?;

    let token: TokenResponse = res.json().await?;
    let refresh_token = token
        .refresh_token
        .ok_or_else(|| anyhow!("spotify token response missing refresh_token"))?;
    let expires_at = unix_now()? + token.expires_in;
    Ok(AuthTokens {
        access_token: token.access_token,
        refresh_token,
        expires_at,
    })
}

async fn refresh_tokens(client_id: &str, client_secret: &str, refresh_token: &str) -> Result<AuthTokens> {
    let client = Client::new();
    let mut form = vec![
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret));
    }
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .form(&form)
        .send()
        .await?
        .error_for_status()?;

    let token: TokenResponse = res.json().await?;
    let expires_at = unix_now()? + token.expires_in;
    Ok(AuthTokens {
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string()),
        expires_at,
    })
}

fn wait_for_callback(captured: Arc<Mutex<Option<String>>>) -> Result<String> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(300) {
        if let Some(url) = captured.lock().unwrap().clone() {
            return Ok(url);
        }
        print!("callback url> ");
        let _ = std::io::stdout().flush();
        let mut buf = String::new();
        if std::io::stdin().read_line(&mut buf).is_ok() {
            let trimmed = buf.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(anyhow!("timed out waiting for spotify callback"))
}

fn read_http_request(stream: &mut TcpStream) -> Option<String> {
    let mut buf = [0u8; 4096];
    let size = stream.read(&mut buf).ok()?;
    let req = String::from_utf8_lossy(&buf[..size]);
    let line = req.lines().next()?;
    let mut parts = line.split_whitespace();
    let _method = parts.next()?;
    let target = parts.next()?;
    let url = format!("http://127.0.0.1{target}");
    Some(url)
}

fn respond_html(stream: &mut TcpStream, body: &str) -> Result<()> {
    let html = format!("<html><body>{}</body></html>", body);
    let payload = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn load_cached_tokens() -> Result<Option<TokenCache>> {
    let path = tokens_path();
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    match serde_yaml::from_str(&raw) {
        Ok(cache) => Ok(Some(cache)),
        Err(_) => Ok(None),
    }
}

fn save_cached_tokens(tokens: &AuthTokens) -> Result<()> {
    let path = tokens_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cache = TokenCache {
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        expires_at: tokens.expires_at,
    };
    fs::write(path, serde_yaml::to_string(&cache)?)?;
    Ok(())
}

fn tokens_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/spotifytui/tokens.yml")
}

fn unix_now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| anyhow!(err))?
        .as_secs())
}

fn is_expired(expires_at: u64) -> bool {
    match unix_now() {
        Ok(now) => now + 60 >= expires_at,
        Err(_) => false,
    }
}
