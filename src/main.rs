mod api;
mod app;
mod auth;
mod config;
mod dotenv;
mod setup;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
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
    app.refresh().await?;

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                if app.handle_key(key.code).await? {
                    break;
                }
            }
        }

        app.tick().await?;
    }

    Ok(())
}
