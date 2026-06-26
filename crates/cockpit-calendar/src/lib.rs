use chrono::{DateTime, Local, TimeZone, Timelike};
use cockpit_domain::{Event, GoogleOAuthConfig};
use base64::Engine;
use rand::{RngCore, thread_rng};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct LocalEventsProvider;

impl LocalEventsProvider {
    pub fn events_for_today(events: &[Event]) -> Vec<Event> {
        Self::events_from_minute(events, current_time_minutes())
    }

    pub fn next_events(events: &[Event], limit: usize) -> Vec<Event> {
        Self::events_for_today(events)
            .into_iter()
            .take(limit)
            .collect()
    }

    fn events_from_minute(events: &[Event], now_minutes: u32) -> Vec<Event> {
        let mut filtered = events
            .iter()
            .filter_map(|event| {
                let minutes = parse_hhmm_to_minutes(&event.time)?;
                (minutes >= now_minutes).then_some((minutes, event.clone()))
            })
            .collect::<Vec<_>>();
        filtered.sort_by_key(|(minutes, _)| *minutes);
        filtered.into_iter().map(|(_, event)| event).collect()
    }
}

fn current_time_minutes() -> u32 {
    let now = Local::now();
    now.hour() * 60 + now.minute()
}

fn parse_hhmm_to_minutes(raw: &str) -> Option<u32> {
    let (hours, minutes) = raw.split_once(':')?;
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

#[derive(Debug)]
pub struct GoogleCalendarProvider {
    client: Client,
}

impl Default for GoogleCalendarProvider {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl GoogleCalendarProvider {
    pub fn next_events_today(
        &self,
        oauth: &GoogleOAuthConfig,
        limit: usize,
    ) -> Result<Vec<Event>, GoogleCalendarError> {
        let now = Local::now();
        let now_minutes = now.hour() * 60 + now.minute();
        let today = now.date_naive();
        let access_token = self.fetch_access_token(oauth)?;
        let (time_min, time_max) = today_time_window();
        let (day_min, day_max) = today_full_day_window();
        let mut events = Vec::new();
        let mut calendar_events_count = 0_usize;
        let mut tasks_events_count = 0_usize;

        for calendar_id in &oauth.calendar_ids {
            let mut calendar_events = self.fetch_calendar_events_for_id(
                &access_token,
                calendar_id,
                &time_min,
                &time_max,
                limit,
            )?;
            calendar_events_count += calendar_events.len();
            events.append(&mut calendar_events);
        }

        if oauth.include_tasks_today {
            let mut tasks = self.fetch_google_tasks_today(
                &access_token,
                &day_min,
                &day_max,
                now_minutes,
                today,
            )?;
            tasks_events_count += tasks.len();
            events.append(&mut tasks);
        }

        eprintln!(
            "google agenda loaded calendar={} tasks={} total_before_filter={}",
            calendar_events_count,
            tasks_events_count,
            events.len()
        );

        events.retain(|event| {
            parse_hhmm_to_minutes(&event.time)
                .map(|minutes| minutes >= current_time_minutes())
                .unwrap_or(false)
        });
        events.sort_by(|left, right| left.time.cmp(&right.time));
        events.truncate(limit);
        Ok(events)
    }

    fn fetch_calendar_events_for_id(
        &self,
        access_token: &str,
        calendar_id: &str,
        time_min: &str,
        time_max: &str,
        limit: usize,
    ) -> Result<Vec<Event>, GoogleCalendarError> {
        let max_results = limit.to_string();
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlencoding::encode(calendar_id)
        );

        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .query(&[
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("timeMin", time_min),
                ("timeMax", time_max),
                ("maxResults", &max_results),
                ("fields", "items(summary,description,start(dateTime,date))"),
            ])
            .send()
            .map_err(GoogleCalendarError::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            return Err(GoogleCalendarError::GoogleApi { status, body });
        }

        let payload: GoogleEventsResponse =
            response.json().map_err(GoogleCalendarError::InvalidJson)?;

        Ok(payload
            .items
            .into_iter()
            .filter_map(map_google_event)
            .collect())
    }

    fn fetch_google_tasks_today(
        &self,
        access_token: &str,
        time_min: &str,
        time_max: &str,
        now_minutes: u32,
        today: chrono::NaiveDate,
    ) -> Result<Vec<Event>, GoogleCalendarError> {
        let max_results = "100".to_string();
        let lists_response = self
            .client
            .get("https://tasks.googleapis.com/tasks/v1/users/@me/lists")
            .bearer_auth(access_token)
            .query(&[("maxResults", &max_results)])
            .send()
            .map_err(GoogleCalendarError::Http)?;

        if !lists_response.status().is_success() {
            let status = lists_response.status().as_u16();
            let body = lists_response.text().unwrap_or_default();
            return Err(GoogleCalendarError::GoogleApi { status, body });
        }

        let lists: GoogleTaskListsResponse = lists_response
            .json()
            .map_err(GoogleCalendarError::InvalidJson)?;
        eprintln!("google tasks lists loaded={}", lists.items.len());

        let mut tasks_events = Vec::new();
        for list in lists.items {
            let url = format!(
                "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
                urlencoding::encode(&list.id)
            );
            let tasks_response = self
                .client
                .get(url)
                .bearer_auth(access_token)
                .query(&[
                    ("showCompleted", "false"),
                    ("showHidden", "false"),
                    ("dueMin", time_min),
                    ("dueMax", time_max),
                    ("maxResults", &max_results),
                    ("fields", "items(title,notes,due,status)"),
                ])
                .send()
                .map_err(GoogleCalendarError::Http)?;

            if !tasks_response.status().is_success() {
                let status = tasks_response.status().as_u16();
                let body = tasks_response.text().unwrap_or_default();
                return Err(GoogleCalendarError::GoogleApi { status, body });
            }

            let tasks: GoogleTasksResponse = tasks_response
                .json()
                .map_err(GoogleCalendarError::InvalidJson)?;
            let raw_count = tasks.items.len();
            let mut mapped = Vec::new();
            for (index, task) in tasks.items.into_iter().enumerate() {
                let raw = serde_json::to_string(&task)
                    .unwrap_or_else(|_| "{\"error\":\"task_serialize_failed\"}".to_string());
                eprintln!("google task raw list={} idx={} payload={}", list.id, index, raw);

                if let Some(event) = map_google_task(task, now_minutes, today) {
                    mapped.push(event);
                }
            }
            eprintln!(
                "google tasks list={} raw={} mapped={} (today={} now_minutes={})",
                list.id,
                raw_count,
                mapped.len(),
                today,
                now_minutes
            );
            tasks_events.extend(mapped);
        }

        Ok(tasks_events)
    }

    fn fetch_access_token(
        &self,
        oauth: &GoogleOAuthConfig,
    ) -> Result<String, GoogleCalendarError> {
        let (client_id, client_secret) = read_google_client_credentials(oauth)?;

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", oauth.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .map_err(GoogleCalendarError::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            return Err(GoogleCalendarError::TokenEndpoint { status, body });
        }

        let token: TokenResponse = response
            .json()
            .map_err(GoogleCalendarError::InvalidJson)?;
        Ok(token.access_token)
    }
}

pub fn google_oauth_first_login(
    credentials_path: &Path,
    scope: &str,
) -> Result<String, GoogleCalendarError> {
    let installed = read_installed_oauth_credentials(credentials_path)?;
    let redirect_uri = installed
        .redirect_uris
        .iter()
        .find(|uri| uri.starts_with("http://localhost"))
        .cloned()
        .unwrap_or_else(|| "http://localhost:8765".to_string());
    let redirect_port = extract_redirect_port(&redirect_uri).unwrap_or(8765);
    let redirect_uri = format!("http://localhost:{redirect_port}");

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&access_type=offline&prompt=consent&state={}&code_challenge={}&code_challenge_method=S256",
        installed.auth_uri,
        urlencoding::encode(&installed.client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scope),
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge)
    );

    eprintln!("Abra esta URL para autorizar o Cockpit:\n{auth_url}\n");
    open_authorization_url(&auth_url);

    let code = wait_for_authorization_code(redirect_port, &state)?;

    let client = Client::new();
    let response = client
        .post(&installed.token_uri)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", installed.client_id.as_str()),
            ("client_secret", installed.client_secret.as_str()),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .map_err(GoogleCalendarError::Http)?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        return Err(GoogleCalendarError::TokenEndpoint { status, body });
    }

    let token: LoginTokenResponse = response
        .json()
        .map_err(GoogleCalendarError::InvalidJson)?;

    token
        .refresh_token
        .ok_or(GoogleCalendarError::MissingRefreshToken)
}

#[derive(Debug, Error)]
pub enum GoogleCalendarError {
    #[error("failed to call Google APIs: {0}")]
    Http(reqwest::Error),
    #[error("failed to parse Google response: {0}")]
    InvalidJson(reqwest::Error),
    #[error("oauth token endpoint error {status}: {body}")]
    TokenEndpoint { status: u16, body: String },
    #[error("google calendar api error {status}: {body}")]
    GoogleApi { status: u16, body: String },
    #[error("failed to read credentials file {path}: {source}")]
    CredentialsFileRead {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse credentials file {path}: {source}")]
    CredentialsFileParse {
        path: String,
        source: serde_json::Error,
    },
    #[error("credentials file does not contain a valid 'installed' or 'web' client")]
    CredentialsFileMissingClient,
    #[error("missing google oauth client_id/client_secret")]
    MissingInlineClientCredentials,
    #[error("failed to bind localhost callback on port {0}")]
    LocalCallbackBind(u16),
    #[error("timed out waiting for OAuth callback")]
    CallbackTimeout,
    #[error("invalid OAuth callback request")]
    InvalidCallbackRequest,
    #[error("oauth callback state mismatch")]
    CallbackStateMismatch,
    #[error("oauth callback returned error: {0}")]
    CallbackError(String),
    #[error("oauth callback missing authorization code")]
    CallbackMissingCode,
    #[error("oauth login succeeded but no refresh_token was returned")]
    MissingRefreshToken,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct LoginTokenResponse {
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventsResponse {
    #[serde(default)]
    items: Vec<GoogleEventItem>,
}

#[derive(Debug, Deserialize)]
struct GoogleTaskListsResponse {
    #[serde(default)]
    items: Vec<GoogleTaskListItem>,
}

#[derive(Debug, Deserialize)]
struct GoogleTaskListItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTasksResponse {
    #[serde(default)]
    items: Vec<GoogleTaskItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GoogleTaskItem {
    title: Option<String>,
    notes: Option<String>,
    due: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventItem {
    summary: Option<String>,
    description: Option<String>,
    start: GoogleEventStart,
}

#[derive(Debug, Deserialize)]
struct GoogleEventStart {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleCredentialsFile {
    installed: Option<GoogleClientCredentials>,
    web: Option<GoogleClientCredentials>,
}

#[derive(Debug, Deserialize)]
struct GoogleClientCredentials {
    client_id: String,
    client_secret: String,
    #[serde(default = "default_auth_uri")]
    auth_uri: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
    #[serde(default)]
    redirect_uris: Vec<String>,
}

fn default_auth_uri() -> String {
    "https://accounts.google.com/o/oauth2/auth".to_string()
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

fn map_google_event(item: GoogleEventItem) -> Option<Event> {
    let title = item.summary.unwrap_or_else(|| "(Sem titulo)".to_string());

    if let Some(date_time) = item.start.date_time {
        let parsed = DateTime::parse_from_rfc3339(&date_time).ok()?;
        let local = parsed.with_timezone(&Local);
        return Some(Event {
            time: local.format("%H:%M").to_string(),
            title,
            description: item.description,
        });
    }

    if item.start.date.is_some() {
        // Expose all-day events as start-of-day so they stay visible in today's list.
        return Some(Event {
            time: "00:00".to_string(),
            title,
            description: item.description,
        });
    }

    None
}

fn map_google_task(item: GoogleTaskItem, now_minutes: u32, today: chrono::NaiveDate) -> Option<Event> {
    if item.status.as_deref() == Some("completed") {
        return None;
    }

    let due = item.due?;
    let (due_date, due_minutes, time) = google_task_due_display_time(&due)?;
    if due_date != today {
        return None;
    }
    if due_minutes < now_minutes {
        return None;
    }

    let title = item.title.unwrap_or_else(|| "(Tarefa sem titulo)".to_string());

    Some(Event {
        time,
        title: format!("[Task] {title}"),
        description: item.notes,
    })
}

fn google_task_due_display_time(due: &str) -> Option<(chrono::NaiveDate, u32, String)> {
    let due_date = due.split('T').next()?.parse::<chrono::NaiveDate>().ok()?;

    // Google Tasks often stores date-only due values as midnight UTC.
    // Keep these visible during the day by treating them as end-of-day items.
    if due.ends_with("T00:00:00.000Z") || due.ends_with("T00:00:00Z") {
        return Some((due_date, 23 * 60 + 59, "23:59".to_string()));
    }

    let parsed = DateTime::parse_from_rfc3339(due).ok()?;
    let local = parsed.with_timezone(&Local);
    let minutes = local.hour() * 60 + local.minute();
    Some((local.date_naive(), minutes, local.format("%H:%M").to_string()))
}

fn today_time_window() -> (String, String) {
    let now = Local::now();
    let date = now.date_naive();
    let start = now.to_rfc3339();

    let end_naive = date
        .and_hms_opt(23, 59, 59)
        .expect("valid end-of-day timestamp");
    let end = Local
        .from_local_datetime(&end_naive)
        .single()
        .unwrap_or(now)
        .to_rfc3339();

    (start, end)
}

fn today_full_day_window() -> (String, String) {
    let now = Local::now();
    let date = now.date_naive();

    let start_naive = date
        .and_hms_opt(0, 0, 0)
        .expect("valid start-of-day timestamp");
    let start = Local
        .from_local_datetime(&start_naive)
        .single()
        .unwrap_or(now)
        .to_rfc3339();

    let end_naive = date
        .and_hms_opt(23, 59, 59)
        .expect("valid end-of-day timestamp");
    let end = Local
        .from_local_datetime(&end_naive)
        .single()
        .unwrap_or(now)
        .to_rfc3339();

    (start, end)
}

fn read_google_client_credentials(
    oauth: &GoogleOAuthConfig,
) -> Result<(String, String), GoogleCalendarError> {
    if let Some(path) = oauth.credentials_file.as_deref() {
        let raw = fs::read_to_string(path).map_err(|source| GoogleCalendarError::CredentialsFileRead {
            path: path.to_string(),
            source,
        })?;
        let file: GoogleCredentialsFile =
            serde_json::from_str(&raw).map_err(|source| GoogleCalendarError::CredentialsFileParse {
                path: path.to_string(),
                source,
            })?;

        if let Some(client) = file.installed.or(file.web) {
            return Ok((client.client_id, client.client_secret));
        }

        return Err(GoogleCalendarError::CredentialsFileMissingClient);
    }

    let client_id = oauth
        .client_id
        .clone()
        .ok_or(GoogleCalendarError::MissingInlineClientCredentials)?;
    let client_secret = oauth
        .client_secret
        .clone()
        .ok_or(GoogleCalendarError::MissingInlineClientCredentials)?;
    Ok((client_id, client_secret))
}

fn read_installed_oauth_credentials(path: &Path) -> Result<GoogleClientCredentials, GoogleCalendarError> {
    let path_str = path.to_string_lossy().to_string();
    let raw = fs::read_to_string(path).map_err(|source| GoogleCalendarError::CredentialsFileRead {
        path: path_str.clone(),
        source,
    })?;
    let file: GoogleCredentialsFile =
        serde_json::from_str(&raw).map_err(|source| GoogleCalendarError::CredentialsFileParse {
            path: path_str,
            source,
        })?;

    file.installed
        .or(file.web)
        .ok_or(GoogleCalendarError::CredentialsFileMissingClient)
}

fn extract_redirect_port(redirect_uri: &str) -> Option<u16> {
    let host = "http://localhost:";
    if !redirect_uri.starts_with(host) {
        return None;
    }

    let remainder = &redirect_uri[host.len()..];
    let port = remainder.split('/').next().unwrap_or_default();
    port.parse::<u16>().ok()
}

fn generate_code_verifier() -> String {
    let mut bytes = [0_u8; 48];
    thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn generate_state() -> String {
    let mut bytes = [0_u8; 16];
    thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn open_authorization_url(url: &str) {
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn wait_for_authorization_code(port: u16, expected_state: &str) -> Result<String, GoogleCalendarError> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|_| GoogleCalendarError::LocalCallbackBind(port))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| GoogleCalendarError::LocalCallbackBind(port))?;

    let deadline = Instant::now() + Duration::from_secs(180);

    loop {
        if Instant::now() >= deadline {
            return Err(GoogleCalendarError::CallbackTimeout);
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0_u8; 4096];
                let bytes = stream
                    .read(&mut buffer)
                    .map_err(|_| GoogleCalendarError::InvalidCallbackRequest)?;
                let request = String::from_utf8_lossy(&buffer[..bytes]);
                let request_line = request
                    .lines()
                    .next()
                    .ok_or(GoogleCalendarError::InvalidCallbackRequest)?;
                let path = extract_request_path(request_line)
                    .ok_or(GoogleCalendarError::InvalidCallbackRequest)?;

                let result = parse_callback_query(path, expected_state);
                let body = match &result {
                    Ok(_) => "Autenticacao concluida. Pode fechar esta aba.",
                    Err(_) => "Falha na autenticacao. Volte ao terminal para detalhes.",
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());

                return result;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return Err(GoogleCalendarError::InvalidCallbackRequest),
        }
    }
}

fn extract_request_path(request_line: &str) -> Option<&str> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?;
    if method != "GET" {
        return None;
    }
    parts.next()
}

fn parse_callback_query(path: &str, expected_state: &str) -> Result<String, GoogleCalendarError> {
    let query = path
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or(GoogleCalendarError::InvalidCallbackRequest)?;

    let mut code = None;
    let mut state = None;
    let mut oauth_error = None;

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let decoded = urlencoding::decode(value)
            .map(|decoded| decoded.into_owned())
            .unwrap_or_default();
        match key {
            "code" => code = Some(decoded),
            "state" => state = Some(decoded),
            "error" => oauth_error = Some(decoded),
            _ => {}
        }
    }

    if let Some(error) = oauth_error {
        return Err(GoogleCalendarError::CallbackError(error));
    }

    let state = state.ok_or(GoogleCalendarError::CallbackStateMismatch)?;
    if state != expected_state {
        return Err(GoogleCalendarError::CallbackStateMismatch);
    }

    code.ok_or(GoogleCalendarError::CallbackMissingCode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_events_by_time() {
        let events = vec![event("13:30", "Daily"), event("10:00", "Aula Marc")];

        let sorted = LocalEventsProvider::events_from_minute(&events, 0);

        assert_eq!(sorted[0].title, "Aula Marc");
        assert_eq!(sorted[1].title, "Daily");
    }

    #[test]
    fn limits_next_events() {
        let events = vec![event("09:00", "One"), event("10:00", "Two")];

        let next = LocalEventsProvider::events_from_minute(&events, 0)
            .into_iter()
            .take(1)
            .collect::<Vec<_>>();

        assert_eq!(next.len(), 1);
        assert_eq!(next[0].title, "One");
    }

    #[test]
    fn filters_only_upcoming_local_events() {
        let events = vec![
            event("08:30", "Past"),
            event("09:00", "Now"),
            event("09:15", "Future"),
        ];

        let filtered = LocalEventsProvider::events_from_minute(&events, 9 * 60);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].title, "Now");
        assert_eq!(filtered[1].title, "Future");
    }

    #[test]
    fn maps_google_datetime_event() {
        let mapped = map_google_event(GoogleEventItem {
            summary: Some("Daily".to_string()),
            description: Some("Sync".to_string()),
            start: GoogleEventStart {
                date_time: Some("2026-06-26T13:30:00-03:00".to_string()),
                date: None,
            },
        })
        .unwrap();

        assert_eq!(mapped.title, "Daily");
        assert_eq!(mapped.time, "13:30");
    }

    #[test]
    fn keeps_midnight_for_google_task_date_due() {
        let (date, minutes, time) =
            google_task_due_display_time("2026-06-26T00:00:00.000Z").unwrap();
        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2026, 6, 26).unwrap());
        assert_eq!(minutes, 23 * 60 + 59);
        assert_eq!(time, "23:59");
    }

    #[test]
    fn keeps_upcoming_google_task_with_explicit_time() {
        let due = "2026-06-26T21:00:00.000Z";
        let task = GoogleTaskItem {
            title: Some("Enviar relatorio".to_string()),
            notes: None,
            due: Some(due.to_string()),
            status: Some("needsAction".to_string()),
        };

        let mapped = map_google_task(
            task,
            0,
            DateTime::parse_from_rfc3339(due)
                .unwrap()
                .with_timezone(&Local)
                .date_naive(),
        );
        assert!(mapped.is_some());
    }

    #[test]
    fn maps_google_all_day_event() {
        let mapped = map_google_event(GoogleEventItem {
            summary: Some("Feriado".to_string()),
            description: None,
            start: GoogleEventStart {
                date_time: None,
                date: Some("2026-06-26".to_string()),
            },
        })
        .unwrap();

        assert_eq!(mapped.time, "00:00");
        assert_eq!(mapped.title, "Feriado");
    }

    #[test]
    fn reads_google_credentials_from_web_section() {
        let oauth = GoogleOAuthConfig {
            calendar_id: "primary".to_string(),
            calendar_ids: vec!["primary".to_string()],
            include_tasks_today: false,
            credentials_file: None,
            client_id: Some("client-id".to_string()),
            client_secret: Some("client-secret".to_string()),
            refresh_token: "refresh".to_string(),
        };

        let credentials = read_google_client_credentials(&oauth).unwrap();
        assert_eq!(credentials.0, "client-id");
        assert_eq!(credentials.1, "client-secret");
    }

    #[test]
    fn parses_callback_query_code_and_state() {
        let code = parse_callback_query("/?code=abc123&state=xyz", "xyz").unwrap();
        assert_eq!(code, "abc123");
    }

    #[test]
    fn rejects_callback_state_mismatch() {
        let error = parse_callback_query("/?code=abc123&state=wrong", "xyz").unwrap_err();
        assert!(matches!(error, GoogleCalendarError::CallbackStateMismatch));
    }

    fn event(time: &str, title: &str) -> Event {
        Event {
            time: time.to_string(),
            title: title.to_string(),
            description: None,
        }
    }
}
