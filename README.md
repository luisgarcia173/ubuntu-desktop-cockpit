# Desktop Cockpit

Desktop Cockpit is a lightweight native Linux desktop overlay for development and mentoring routines.

The project is intentionally native Rust: no Electron, no browser engine, no Tauri, and no WebView.

## Current Scope

- Cargo workspace with separated crates.
- TOML configuration loader.
- Markdown checkbox task parser.
- Local event loading from config.
- CPU/RAM metrics provider.
- Testable dashboard use case.
- GTK4 UI code behind the optional `gtk-ui` feature.

## Requirements

For the non-GTK core:

```bash
cargo check
cargo test
```

For the GTK UI on Ubuntu:

```bash
sudo apt install libgtk-4-dev
```

## Running

Run in console mode:

```bash
cargo run -p cockpit-app -- --config examples/config.example.toml
```

Run in GTK UI mode:

```bash
cargo run -p cockpit-app --features gtk-ui -- --config examples/config.example.toml
```

Run GTK UI with auto-restart on file changes:

```bash
cargo install cargo-watch
cargo watch -x "run -p cockpit-app --features gtk-ui -- --config examples/config.example.toml"
```

Without the `gtk-ui` feature, `cockpit-app` prints the dashboard model to stdout.

## Configuration

Desktop Cockpit searches config in this order:

1. `--config <path>`
2. `~/.config/desktop-cockpit/config.toml`
3. `examples/config.example.toml` during development

See [examples/config.example.toml](examples/config.example.toml).

### Google Calendar OAuth

The `Proximos eventos` section can read Google Calendar events from the current day with
start time greater than or equal to now.

1. Create OAuth credentials in Google Cloud Console for a desktop app.
2. Generate a refresh token for the scope `https://www.googleapis.com/auth/calendar.readonly`.
3. Update your config:

```toml
[calendar]
provider = "google-oauth"

[calendar.google_oauth]
calendar_id = "primary"
calendar_ids = ["primary", "work@example.com"]
include_tasks_today = true
credentials_file = "credentials.json"
refresh_token = "your-google-refresh-token"
```

`credentials.json` must be the Google OAuth client file (Desktop app) containing
an `installed` (or `web`) block with `client_id` and `client_secret`.

Generate the refresh token directly with the app (first login):

```bash
cargo run -p cockpit-app -- --google-login --credentials credentials/credentials.json
```

After browser consent, copy the printed refresh token to `calendar.google_oauth.refresh_token`.

The first-login flow requests read-only scopes for both Calendar and Tasks:

- `https://www.googleapis.com/auth/calendar.readonly`
- `https://www.googleapis.com/auth/tasks.readonly`

When `provider = "local"`, the app keeps using `[[events]]` entries from TOML.

## Roadmap

- V0.1: GTK window, TOML config, clock, local events, Markdown tasks, CPU/RAM, basic CSS.
- V0.2: refined overlay positioning, monitor selection, shortcuts, autostart.
- V0.3: local `.ics`, container status, local service status.
- V0.4: themes, simple config editor, `.deb` packaging.

## Known Limitations

- GNOME/Wayland does not expose reliable monitor targeting for normal GTK4 windows, so monitor switching is not implemented.
- Microsoft login, telemetry, and AI are intentionally out of V0.1.
