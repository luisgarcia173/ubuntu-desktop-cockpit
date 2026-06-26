use std::path::{Path, PathBuf};

use cockpit_calendar::LocalEventsProvider;
use cockpit_config::ConfigLoader;
use cockpit_core::{
    BuildDashboardError, BuildDashboardUseCase, EventsProvider, MetricsProvider, TasksProvider,
};
use cockpit_domain::{AppConfig, Event, SystemMetric, Task};
use cockpit_notes::MarkdownTasksProvider;
use cockpit_system::SystemMetricsProvider;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config_path = parse_config_path(std::env::args().skip(1))?;
    let config = ConfigLoader::load(config_path.as_deref()).map_err(|error| error.to_string())?;

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

fn parse_config_path(args: impl IntoIterator<Item = String>) -> Result<Option<PathBuf>, String> {
    let mut args = args.into_iter();
    let mut config_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--config requires a path".to_string())?;
                config_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("Usage: cockpit-app [--config <path>]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(config_path)
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
        Ok(LocalEventsProvider::next_events(&config.events, 5))
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
