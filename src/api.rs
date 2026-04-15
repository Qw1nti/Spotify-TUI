use anyhow::Result;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct SpotifyApi {
    client: Client,
    access_token: String,
}

impl SpotifyApi {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    pub async fn me(&self) -> Result<SpotifyUser> {
        self.get("https://api.spotify.com/v1/me").await
    }

    pub async fn current_playback(&self) -> Result<Option<PlayerState>> {
        let resp = self.request(Method::GET, "https://api.spotify.com/v1/me/player").send().await?;
        if resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }
        Ok(Some(resp.error_for_status()?.json().await?))
    }

    pub async fn search_tracks(&self, query: &str, limit: usize) -> Result<SearchResponse> {
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/search",
            &[
                ("q", query),
                ("type", "track"),
                ("limit", &limit.to_string()),
            ],
        )?;
        self.get(url.as_str()).await
    }

    pub async fn followed_artists(&self, limit: usize) -> Result<ArtistsPage> {
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/me/following",
            &[("type", "artist"), ("limit", &limit.to_string())],
        )?;
        self.get(url.as_str()).await
    }

    pub async fn liked_tracks(&self, limit: usize) -> Result<SavedTracksPage> {
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/me/tracks",
            &[("limit", &limit.to_string())],
        )?;
        self.get(url.as_str()).await
    }

    pub async fn playlists(&self, limit: usize) -> Result<PlaylistsPage> {
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/me/playlists",
            &[("limit", &limit.to_string())],
        )?;
        self.get(url.as_str()).await
    }

    pub async fn recent_tracks(&self, limit: usize) -> Result<RecentTracksPage> {
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/me/player/recently-played",
            &[("limit", &limit.to_string())],
        )?;
        self.get(url.as_str()).await
    }

    pub async fn devices(&self) -> Result<DevicePage> {
        self.get("https://api.spotify.com/v1/me/player/devices").await
    }

    pub async fn toggle_playback(&self) -> Result<()> {
        let playback = self.current_playback().await?;
        if let Some(state) = playback {
            if state.is_playing {
                self.request(Method::PUT, "https://api.spotify.com/v1/me/player/pause")
                    .send()
                    .await?
                    .error_for_status()?;
            } else {
                self.request(Method::PUT, "https://api.spotify.com/v1/me/player/play")
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }
        Ok(())
    }

    pub async fn next_track(&self) -> Result<()> {
        self.request(Method::POST, "https://api.spotify.com/v1/me/player/next")
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn previous_track(&self) -> Result<()> {
        self.request(Method::POST, "https://api.spotify.com/v1/me/player/previous")
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn queue_track(&self, uri: &str) -> Result<()> {
        self.request(
            Method::POST,
            &format!("https://api.spotify.com/v1/me/player/queue?uri={}", urlencoding::encode(uri)),
        )
        .send()
        .await?
        .error_for_status()?;
        Ok(())
    }

    pub async fn play_track(&self, uri: &str) -> Result<()> {
        self.request(Method::PUT, "https://api.spotify.com/v1/me/player/play")
            .json(&PlayRequest {
                uris: vec![uri.to_string()],
            })
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        Ok(self
            .request(Method::GET, url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    fn request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.access_token)
    }
}

#[derive(Debug, Serialize)]
struct PlayRequest {
    uris: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpotifyUser {
    pub display_name: Option<String>,
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlayerState {
    pub is_playing: bool,
    pub progress_ms: Option<u32>,
    pub item: Option<Track>,
    pub device: Option<Device>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Device {
    pub name: String,
    pub is_active: bool,
    pub volume_percent: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Track {
    pub name: String,
    pub uri: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration_ms: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Artist {
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Album {
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchResponse {
    pub tracks: Paging<Track>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paging<T> {
    pub items: Vec<T>,
    pub total: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArtistsPage {
    pub artists: FollowedArtists,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FollowedArtists {
    pub items: Vec<Artist>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SavedTracksPage {
    pub items: Vec<SavedTrack>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SavedTrack {
    pub track: Track,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlaylistsPage {
    pub items: Vec<Playlist>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Playlist {
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecentTracksPage {
    pub items: Vec<RecentTrack>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecentTrack {
    pub track: Track,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DevicePage {
    pub devices: Vec<Device>,
}
