use std::path::{Path, PathBuf};

use cockpit_calendar::{GoogleCalendarProvider, LocalEventsProvider, google_oauth_first_login};
use cockpit_config::ConfigLoader;
use cockpit_core::{
    BuildDashboardError, BuildDashboardUseCase, EventsProvider, MetricsProvider, TasksProvider,
};
use cockpit_domain::{AppConfig, CalendarProvider, Event, SystemMetric, Task};
use cockpit_notes::MarkdownTasksProvider;
use cockpit_system::SystemMetricsProvider;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match parse_cli_command(std::env::args().skip(1))? {
        CliCommand::RunDashboard { config_path } => run_dashboard(config_path.as_deref()),
        CliCommand::GoogleLogin { credentials_path } => run_google_login(&credentials_path),
    }
}

fn run_dashboard(config_path: Option<&Path>) -> Result<(), String> {
    let config = ConfigLoader::load(config_path).map_err(|error| error.to_string())?;

    let tasks = LocalTasksAdapter;
    let events = LocalEventsAdapter;
    let metrics = SystemMetricsAdapter::default();
    let mut use_case = BuildDashboardUseCase::new(tasks, events, metrics);
    let dashboard = use_case
        .execute(&config)
        .map_err(|error| error.to_string())?;

    cockpit_ui_gtk::run_dashboard(&config, dashboard);

    Ok(())
}

fn run_google_login(credentials_path: &Path) -> Result<(), String> {
    let refresh_token = google_oauth_first_login(
        credentials_path,
        "https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/tasks.readonly",
    )
    .map_err(|error| error.to_string())?;

    println!("Refresh token gerado com sucesso:\n{refresh_token}");
    Ok(())
}

enum CliCommand {
    RunDashboard { config_path: Option<PathBuf> },
    GoogleLogin { credentials_path: PathBuf },
}

fn parse_cli_command(args: impl IntoIterator<Item = String>) -> Result<CliCommand, String> {
    let mut args = args.into_iter();
    let mut config_path = None;
    let mut credentials_path = PathBuf::from("credentials/credentials.json");
    let mut google_login = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--config requires a path".to_string())?;
                config_path = Some(PathBuf::from(value));
            }
            "--credentials" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--credentials requires a path".to_string())?;
                credentials_path = PathBuf::from(value);
            }
            "--google-login" => {
                google_login = true;
            }
            "--help" | "-h" => {
                println!(
                    "Usage:\n  cockpit-app [--config <path>]\n  cockpit-app --google-login [--credentials <path>]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if google_login {
        Ok(CliCommand::GoogleLogin { credentials_path })
    } else {
        Ok(CliCommand::RunDashboard { config_path })
    }
}

struct LocalTasksAdapter;

impl TasksProvider for LocalTasksAdapter {
    fn load_tasks(&self, config: &AppConfig) -> Result<Vec<Task>, BuildDashboardError> {
        let Some(path) = config.notes.daily_file.as_deref() else {
            return Ok(Vec::new());
        };

        MarkdownTasksProvider::load_pending_top3(Path::new(path))
            .map_err(|error| BuildDashboardError::Tasks(error.to_string()))
    }
}

struct LocalEventsAdapter;

impl EventsProvider for LocalEventsAdapter {
    fn load_events(&self, config: &AppConfig) -> Result<Vec<Event>, BuildDashboardError> {
        match config.calendar.provider {
            CalendarProvider::Local => {
                let events = LocalEventsProvider::next_events(&config.events, 5);
                log_loaded_events("local", &events);
                Ok(events)
            }
            CalendarProvider::GoogleOAuth => {
                let Some(oauth) = config.calendar.google_oauth.as_ref() else {
                    eprintln!(
                        "calendar provider is google-oauth but credentials are missing; using local events fallback"
                    );
                    let events = LocalEventsProvider::next_events(&config.events, 5);
                    log_loaded_events("local-fallback", &events);
                    return Ok(events);
                };

                match GoogleCalendarProvider::default().next_events_today(oauth, 5) {
                    Ok(events) => {
                        log_loaded_events("google-calendar", &events);
                        Ok(events)
                    }
                    Err(error) => {
                        eprintln!(
                            "failed to load Google Calendar events: {error}; using local events fallback"
                        );
                        let events = LocalEventsProvider::next_events(&config.events, 5);
                        log_loaded_events("local-fallback", &events);
                        Ok(events)
                    }
                }
            }
        }
    }
}

fn log_loaded_events(source: &str, events: &[Event]) {
    eprintln!("calendar source `{source}` loaded {} event(s)", events.len());
    for event in events {
        eprintln!("  - {}  {}", event.time, event.title);
    }
}

#[derive(Default)]
struct SystemMetricsAdapter {
    provider: SystemMetricsProvider,
}

impl MetricsProvider for SystemMetricsAdapter {
    fn load_metrics(&mut self) -> Result<SystemMetric, BuildDashboardError> {
        self.provider
            .collect()
            .map_err(|error| BuildDashboardError::System(error.to_string()))
    }
}
