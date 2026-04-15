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

    pub async fn devices(&self) -> Result<DevicePage> {
        self.get("https://api.spotify.com/v1/me/player/devices").await
    }

    pub async fn toggle_playback(&self, device_id: Option<&str>) -> Result<()> {
        let playback = self.current_playback().await?;
        if let Some(state) = playback {
            if state.is_playing {
                self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/pause", device_id)
                    .send()
                    .await?
                    .error_for_status()?;
            } else {
                self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/play", device_id)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }
        Ok(())
    }

    pub async fn next_track(&self, device_id: Option<&str>) -> Result<()> {
        self.player_command(Method::POST, "https://api.spotify.com/v1/me/player/next", device_id)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn previous_track(&self, device_id: Option<&str>) -> Result<()> {
        self.player_command(Method::POST, "https://api.spotify.com/v1/me/player/previous", device_id)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn queue_track(&self, uri: &str, device_id: Option<&str>) -> Result<()> {
        let mut url = reqwest::Url::parse("https://api.spotify.com/v1/me/player/queue")?;
        url.query_pairs_mut().append_pair("uri", uri);
        if let Some(device_id) = device_id {
            url.query_pairs_mut().append_pair("device_id", device_id);
        }
        self.request(Method::POST, url.as_str())
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn transfer_playback(&self, device_id: &str, play: bool) -> Result<()> {
        self.client
            .request(Method::PUT, "https://api.spotify.com/v1/me/player")
            .bearer_auth(&self.access_token)
            .json(&TransferPlaybackRequest {
                device_ids: vec![device_id.to_string()],
                play,
            })
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn play_track(&self, uri: &str, device_id: Option<&str>) -> Result<()> {
        self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/play", device_id)
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

    fn player_command(&self, method: Method, url: &str, device_id: Option<&str>) -> reqwest::RequestBuilder {
        let mut request = self.request(method, url);
        if let Some(device_id) = device_id {
            request = request.query(&[("device_id", device_id)]);
        }
        request
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
    pub id: Option<String>,
    pub name: String,
    pub is_active: bool,
    pub volume_percent: Option<u32>,
}

#[derive(Debug, Serialize)]
struct TransferPlaybackRequest {
    device_ids: Vec<String>,
    play: bool,
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
pub struct DevicePage {
    pub devices: Vec<Device>,
}
