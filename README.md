# spotifytui

Rust rewrite of Spotify TUI with a cleaner terminal UI and loopback OAuth on `http://127.0.0.1:8890/callback`.

## What this build does

- Uses a single Rust binary named `spt`
- Uses `127.0.0.1:8890` redirect URI by default
- Ships a new TUI layout with left navigation, main content, detail pane, and footer hints
- Covers core Spotify flows: auth, now playing, search, liked tracks, playlists, and devices

## Config

Config lives at `~/.config/spotifytui/config.yml`.
Cached auth tokens live at `~/.config/spotifytui/tokens.yml`.
First launch runs a setup wizard that asks for Spotify app id, app secret, and confirms redirect URI.
Register `http://127.0.0.1:8890/callback` in Spotify dashboard.
The secret prompt is hidden while you type. Leave it blank if you want PKCE-only auth.
Optional `.env` file in repo root can hold `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET`, and `SPOTIFY_CALLBACK_URL`.
`SPOTIFY_CALLBACK_URL` should be full browser redirect URL with `code` and `state` query params if you want to bypass the prompt.

Example:

```yaml
client_id: "your-spotify-client-id"
redirect_hosts:
  - "127.0.0.1"
redirect_ports:
  - 8890
theme:
  accent: "Cyan"
  accent_soft: "LightCyan"
  background: "Black"
  surface: "DarkGray"
  text: "White"
  muted: "Gray"
  danger: "LightRed"
  success: "LightGreen"
ui:
  list_page_size: 20
```

## Run

```bash
cargo run --bin spt
```

If Spotify auth fails in browser, paste callback URL into terminal when prompted.
If Spotify returns `error=server_error`, re-check that the redirect URI in the Spotify dashboard is exactly `http://127.0.0.1:8890/callback`.
To run by typing `spotifytui`, put repo root on `PATH` or symlink/copy the `spotifytui` launcher script into a PATH directory.

## Installer

For a curl-based install from GitHub, run:

```bash
curl -fsSL https://raw.githubusercontent.com/Qw1nti/Spotify-TUI/main/install.sh | bash
```

Installer downloads `Qw1nti/Spotify-TUI`, asks whether to install globally, then prints `spotifytui`. After install, run `spotifytui`.
If you want to remove it later, run `./uninstall.sh` from the repo root. That removes the installed `spotifytui` command and deletes the app config/token directories.

## First Run

1. Open installer or run `spotifytui` from repo root.
2. Enter Spotify app id and app secret when prompted.
3. Add `http://127.0.0.1:8890/callback` to Spotify app redirect URIs.
4. Sign in once in browser.
5. Future launches use cached token until it expires.

## Keys

- `n` next track
- `b` previous track
- `/` search
- `enter` search or play selected track
- `a` queue selected track
- `o` play selected track now
- `space` toggle playback
- `Esc` keep current search input without re-querying

## License

MIT. See [LICENSE](/mnt/c/Users/ian/Documents/CodeStuff/SpotifyTUI/LICENSE).
