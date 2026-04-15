use anyhow::Result;
use reqwest::{Client, Method, Response, StatusCode};
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
        let resp = self
            .request(Method::GET, "https://api.spotify.com/v1/me/player")
            .send()
            .await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let resp = self.checked_response(resp, "read current playback", "https://api.spotify.com/v1/me/player")?;
        Ok(Some(resp.json().await?))
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
                self.send_player_command(
                    self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/pause", device_id)
                        .json(&serde_json::json!({})),
                    "pause playback",
                    "https://api.spotify.com/v1/me/player/pause",
                )
                .await?;
            } else {
                self.send_player_command(
                    self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/play", device_id)
                        .json(&serde_json::json!({})),
                    "resume playback",
                    "https://api.spotify.com/v1/me/player/play",
                )
                .await?;
            }
        }
        Ok(())
    }

    pub async fn next_track(&self, device_id: Option<&str>) -> Result<()> {
        self.send_player_command(
            self.player_command(Method::POST, "https://api.spotify.com/v1/me/player/next", device_id)
                .json(&serde_json::json!({})),
            "skip to next track",
            "https://api.spotify.com/v1/me/player/next",
        )
        .await?;
        Ok(())
    }

    pub async fn previous_track(&self, device_id: Option<&str>) -> Result<()> {
        self.send_player_command(
            self.player_command(Method::POST, "https://api.spotify.com/v1/me/player/previous", device_id)
                .json(&serde_json::json!({})),
            "skip to previous track",
            "https://api.spotify.com/v1/me/player/previous",
        )
        .await?;
        Ok(())
    }

    pub async fn queue_track(&self, uri: &str, device_id: Option<&str>) -> Result<()> {
        let mut url = reqwest::Url::parse("https://api.spotify.com/v1/me/player/queue")?;
        url.query_pairs_mut().append_pair("uri", uri);
        if let Some(device_id) = device_id {
            url.query_pairs_mut().append_pair("device_id", device_id);
        }
        self.send_player_command(
            self.request(Method::POST, url.as_str()),
            "queue track",
            "https://api.spotify.com/v1/me/player/queue",
        )
        .await?;
        Ok(())
    }

    pub async fn transfer_playback(&self, device_id: &str, play: bool) -> Result<()> {
        self.send_player_command(
            self.client
                .request(Method::PUT, "https://api.spotify.com/v1/me/player")
                .bearer_auth(&self.access_token)
                .json(&TransferPlaybackRequest {
                    device_ids: vec![device_id.to_string()],
                    play,
                }),
            "transfer playback",
            "https://api.spotify.com/v1/me/player",
        )
        .await?;
        Ok(())
    }

    pub async fn play_track(&self, uri: &str, device_id: Option<&str>) -> Result<()> {
        self.send_player_command(
            self.player_command(Method::PUT, "https://api.spotify.com/v1/me/player/play", device_id).json(&PlayRequest {
                uris: vec![uri.to_string()],
            }),
            "start playback",
            "https://api.spotify.com/v1/me/player/play",
        )
        .await?;
        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let resp = self.request(Method::GET, url).send().await?;
        let resp = self.checked_response(resp, "fetch data", url)?;
        Ok(resp.json().await?)
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

    async fn send_player_command(&self, request: reqwest::RequestBuilder, action: &str, url: &str) -> Result<()> {
        let resp = request.send().await?;
        self.checked_response(resp, action, url)?;
        Ok(())
    }

    fn checked_response(&self, resp: Response, action: &str, url: &str) -> Result<Response> {
        if resp.status().is_success() {
            return Ok(resp);
        }

        let status = resp.status();
        Err(status_error(status, action, url))
    }
}

fn status_error(status: StatusCode, action: &str, url: &str) -> anyhow::Error {
    let message = match status.as_u16() {
        401 => format!("{action} failed: Spotify auth expired. Run `spotifytui onboard` again."),
        403 => format!("{action} failed: Spotify denied access. Open Spotify on a device and try again."),
        404 => format!("{action} failed: no active Spotify device found. Open Spotify, select a device, then try again."),
        411 => format!("{action} failed: Spotify rejected an empty request body. Update the app or try again after refresh."),
        429 => format!("{action} failed: Spotify rate limited the request. Try again in a moment."),
        code if (500..600).contains(&code) => format!("{action} failed: Spotify service error ({status}). Try again later."),
        _ => format!("{action} failed: HTTP {status} from {url}"),
    };
    anyhow::anyhow!(message)
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
