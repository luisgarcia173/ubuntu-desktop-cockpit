use chrono::Local;
use cockpit_domain::{AppConfig, ClockInfo, Dashboard, Event, SystemMetric, Task};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildDashboardError {
    #[error("failed to load tasks: {0}")]
    Tasks(String),
    #[error("failed to load events: {0}")]
    Events(String),
    #[error("failed to load system metrics: {0}")]
    System(String),
}

pub trait TasksProvider {
    fn load_tasks(&self, config: &AppConfig) -> Result<Vec<Task>, BuildDashboardError>;
}

pub trait EventsProvider {
    fn load_events(&self, config: &AppConfig) -> Result<Vec<Event>, BuildDashboardError>;
}

pub trait MetricsProvider {
    fn load_metrics(&mut self) -> Result<SystemMetric, BuildDashboardError>;
}

pub struct BuildDashboardUseCase<Tasks, Events, Metrics> {
    tasks: Tasks,
    events: Events,
    metrics: Metrics,
}

impl<Tasks, Events, Metrics> BuildDashboardUseCase<Tasks, Events, Metrics>
where
    Tasks: TasksProvider,
    Events: EventsProvider,
    Metrics: MetricsProvider,
{
    pub fn new(tasks: Tasks, events: Events, metrics: Metrics) -> Self {
        Self {
            tasks,
            events,
            metrics,
        }
    }

    pub fn execute(&mut self, config: &AppConfig) -> Result<Dashboard, BuildDashboardError> {
        let now = Local::now();

        Ok(Dashboard {
            app_name: config.name.clone(),
            sections: config.sections.clone(),
            clock: ClockInfo {
                time: now.format("%H:%M").to_string(),
                date: now.format("%A, %d %b").to_string(),
            },
            events: self.events.load_events(config)?,
            tasks: self.tasks.load_tasks(config)?,
            system: self.metrics.load_metrics()?,
            shortcuts: config.shortcuts.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cockpit_domain::{DashboardSections, Event, Shortcut};

    #[test]
    fn builds_dashboard_with_fakes() {
        let config = AppConfig {
            name: "Test".to_string(),
            sections: DashboardSections::default(),
            shortcuts: vec![Shortcut {
                label: "Terminal".to_string(),
                command: "ghostty".to_string(),
            }],
            ..AppConfig::default()
        };
        let mut use_case = BuildDashboardUseCase::new(FakeTasks, FakeEvents, FakeMetrics);

        let dashboard = use_case.execute(&config).unwrap();

        assert_eq!(dashboard.app_name, "Test");
        assert_eq!(dashboard.tasks[0].title, "Task");
        assert_eq!(dashboard.events[0].title, "Event");
        assert_eq!(dashboard.system.memory_usage_percent, 50.0);
        assert_eq!(dashboard.shortcuts[0].label, "Terminal");
    }

    struct FakeTasks;

    impl TasksProvider for FakeTasks {
        fn load_tasks(&self, _config: &AppConfig) -> Result<Vec<Task>, BuildDashboardError> {
            Ok(vec![Task {
                title: "Task".to_string(),
                completed: false,
            }])
        }
    }

    struct FakeEvents;

    impl EventsProvider for FakeEvents {
        fn load_events(&self, _config: &AppConfig) -> Result<Vec<Event>, BuildDashboardError> {
            Ok(vec![Event {
                time: "10:00".to_string(),
                title: "Event".to_string(),
                description: None,
            }])
        }
    }

    struct FakeMetrics;

    impl MetricsProvider for FakeMetrics {
        fn load_metrics(&mut self) -> Result<SystemMetric, BuildDashboardError> {
            Ok(SystemMetric {
                cpu_usage_percent: 10.0,
                memory_total_bytes: 100,
                memory_used_bytes: 50,
                memory_usage_percent: 50.0,
            })
        }
    }
}
