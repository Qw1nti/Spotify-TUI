use anyhow::Result;
use crossterm::event::KeyCode;

use crate::{api, config::Config};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Home,
    Search,
    Library,
    Playlists,
    Devices,
}

impl Section {
    fn all() -> &'static [Section] {
        &[
            Section::Home,
            Section::Search,
            Section::Library,
            Section::Playlists,
            Section::Devices,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            Section::Home => "Now Playing",
            Section::Search => "Search",
            Section::Library => "Liked",
            Section::Playlists => "Playlists",
            Section::Devices => "Devices",
        }
    }
}

pub struct App {
    pub config: Config,
    pub api: api::SpotifyApi,
    pub section: Section,
    pub user: Option<api::SpotifyUser>,
    pub playback: Option<api::PlayerState>,
    pub search_query: String,
    pub search_dirty: bool,
    pub search_results: Vec<api::Track>,
    pub search_view_offset: usize,
    pub liked_tracks: Vec<api::Track>,
    pub playlists: Vec<api::Playlist>,
    pub devices: Vec<api::Device>,
    pub status: String,
    pub selected: usize,
}

impl App {
    pub fn new(config: Config, api: api::SpotifyApi) -> Self {
        Self {
            config,
            api,
            section: Section::Home,
            user: None,
            playback: None,
            search_query: String::new(),
            search_dirty: false,
            search_results: Vec::new(),
            search_view_offset: 0,
            liked_tracks: Vec::new(),
            playlists: Vec::new(),
            devices: Vec::new(),
            status: "Ready".into(),
            selected: 0,
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.user = self.api.me().await.ok();
        self.playback = self.api.current_playback().await.ok().flatten();
        self.devices = self
            .api
            .devices()
            .await
            .map(|p| p.devices)
            .unwrap_or_default();
        self.liked_tracks = self
            .api
            .liked_tracks(self.config.ui.list_page_size)
            .await
            .map(|p| p.items.into_iter().map(|s| s.track).collect())
            .unwrap_or_default();
        self.playlists = self
            .api
            .playlists(self.config.ui.list_page_size)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
        Ok(())
    }

    pub async fn handle_key(&mut self, code: KeyCode) -> Result<bool> {
        if self.section == Section::Search {
            match code {
                KeyCode::Enter => {
                    if self.search_dirty || self.search_results.is_empty() {
                        let resp = self.api.search_tracks(&self.search_query, self.config.ui.list_page_size).await?;
                        self.search_results = resp.tracks.items;
                        self.selected = 0;
                        self.search_view_offset = 0;
                        self.search_dirty = false;
                        self.status = format!("Found {} tracks", self.search_results.len());
                    } else if let Some(uri) = self.selected_track_uri() {
                        self.api.play_track(uri).await?;
                        self.playback = self.api.current_playback().await.ok().flatten();
                        self.status = "Playing selected track".into();
                    }
                    self.clamp_selection();
                    return Ok(false);
                }
                KeyCode::Esc => {
                    self.search_dirty = false;
                    self.status = "Search input unchanged".into();
                    return Ok(false);
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.search_dirty = true;
                    self.clamp_selection();
                    return Ok(false);
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.search_dirty = true;
                    self.clamp_selection();
                    return Ok(false);
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Char('q') if self.section != Section::Search => return Ok(true),
            KeyCode::F(1) => {
                self.status = "j/k or arrows move. tab switches section. / focus search. space toggles playback. F1 help.".into();
            }
            KeyCode::Tab => self.next_section(),
            KeyCode::BackTab => self.prev_section(),
            KeyCode::Down | KeyCode::Char('j') => self.selected = self.selected.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('n') => {
                self.api.next_track().await?;
                self.playback = self.api.current_playback().await.ok().flatten();
                self.status = "Skipped to next track".into();
            }
            KeyCode::Char('b') => {
                self.api.previous_track().await?;
                self.playback = self.api.current_playback().await.ok().flatten();
                self.status = "Went to previous track".into();
            }
            KeyCode::Char('r') => {
                self.refresh().await?;
                self.status = "Refreshed".into();
            }
            KeyCode::Char(' ') => {
                self.api.toggle_playback().await?;
                self.playback = self.api.current_playback().await.ok().flatten();
                self.status = if self.playback.as_ref().map(|p| p.is_playing).unwrap_or(false) {
                    "Playing".into()
                } else {
                    "Paused".into()
                };
            }
            KeyCode::Char('/') => {
                self.section = Section::Search;
                self.status = "Type query in search bar".into();
            }
            KeyCode::Char('a') => {
                if let Some(uri) = self.selected_track_uri() {
                    self.api.queue_track(uri).await?;
                    self.status = "Queued selected track".into();
                }
            }
            KeyCode::Char('o') => {
                if let Some(uri) = self.selected_track_uri() {
                    self.api.play_track(uri).await?;
                    self.playback = self.api.current_playback().await.ok().flatten();
                    self.status = "Playing selected track".into();
                }
            }
            _ => {}
        }

        self.clamp_selection();
        Ok(false)
    }

    pub async fn tick(&mut self) -> Result<()> {
        Ok(())
    }

    fn next_section(&mut self) {
        let idx = Section::all().iter().position(|s| *s == self.section).unwrap_or(0);
        self.section = Section::all()[(idx + 1) % Section::all().len()];
        self.selected = 0;
    }

    fn prev_section(&mut self) {
        let idx = Section::all().iter().position(|s| *s == self.section).unwrap_or(0);
        self.section = Section::all()[(idx + Section::all().len() - 1) % Section::all().len()];
        self.selected = 0;
    }

    fn clamp_selection(&mut self) {
        let max = self.current_items().len();
        if max == 0 {
            self.selected = 0;
        } else if self.selected >= max {
            self.selected = max - 1;
        }
        self.search_view_offset = self.search_view_offset.min(self.selected);
    }

    fn selected_track_uri(&self) -> Option<&str> {
        match self.section {
            Section::Search => self.search_results.get(self.selected).map(|t| t.uri.as_str()),
            Section::Library => self.liked_tracks.get(self.selected).map(|t| t.uri.as_str()),
            Section::Home => self
                .playback
                .as_ref()
                .and_then(|p| p.item.as_ref())
                .map(|t| t.uri.as_str()),
            _ => None,
        }
    }

    pub fn current_items(&self) -> Vec<String> {
        match self.section {
            Section::Home => self.playback
                .as_ref()
                .and_then(|p| p.item.as_ref())
                .map(|t| vec![format_track(t)])
                .unwrap_or_default(),
            Section::Search => self.search_results.iter().map(format_track).collect(),
            Section::Library => self.liked_tracks.iter().map(format_track).collect(),
            Section::Playlists => self.playlists.iter().map(|p| p.name.clone()).collect(),
            Section::Devices => self.devices.iter().map(|d| d.name.clone()).collect(),
        }
    }

    pub fn section_title(&self) -> &'static str {
        self.section.title()
    }

    pub fn empty_state(&self) -> Option<&'static str> {
        match self.section {
            Section::Home => Some("No active playback. Press `o` on a track or `space` to resume."),
            Section::Search if self.search_results.is_empty() => Some("Type a search query and press Enter. Press Esc to keep input and avoid re-search."),
            Section::Library if self.liked_tracks.is_empty() => Some("No liked tracks found."),
            Section::Playlists if self.playlists.is_empty() => Some("No playlists found."),
            Section::Devices if self.devices.is_empty() => Some("No Spotify devices available."),
            _ => None,
        }
    }
}

fn format_track(track: &api::Track) -> String {
    let artists = track
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} - {} / {}", track.name, artists, track.album.name)
}
