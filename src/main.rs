mod api;
mod app;
mod auth;
mod config;
mod dotenv;
mod maintenance;
mod setup;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        maintenance::print_help();
        return Ok(());
    }
    if matches!(args.first().map(|arg| arg.as_str()), Some("help")) {
        maintenance::print_help();
        return Ok(());
    }
    if args.first().map(|arg| arg.as_str()) == Some("onboard") {
        return maintenance::run_onboard();
    }
    if args.first().map(|arg| arg.as_str()) == Some("uninstall") {
        return maintenance::run_uninstall();
    }

    let env = dotenv::Dotenv::load()?;
    let config = setup::ensure_config(&env)?;
    let callback_override = env
        .get("SPOTIFY_CALLBACK_URL")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let auth = auth::authenticate(&config, callback_override).await?;
    let api = api::SpotifyApi::new(auth.access_token);
    let mut app = app::App::new(config, api);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    run_result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    if let Err(err) = app.refresh().await {
        app.status = friendly_error(&err);
    }

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                match app.handle_key(key.code).await {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(err) => {
                        app.status = friendly_error(&err);
                    }
                }
            }
        }

        app.tick().await?;
    }

    Ok(())
}

pub fn home_dir(path: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(path)
}

pub fn config_dir(app: &str) -> PathBuf {
    home_dir(&format!(".config/{app}"))
}

fn friendly_error(err: &anyhow::Error) -> String {
    let message = err.to_string();
    if message.contains("rate limited") {
        return "Spotify rate limited the request. Try again in a moment.".into();
    }
    if message.contains("auth expired") {
        return "Spotify auth expired. Run `spotifytui onboard` again.".into();
    }
    if message.contains("no active Spotify device") {
        return "No active Spotify device found. Open Spotify and choose a device.".into();
    }
    if message.contains("rejected an empty request body") {
        return "Spotify rejected a request body. Refresh and try again.".into();
    }
    message
}
