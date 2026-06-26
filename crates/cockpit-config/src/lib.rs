use std::fs;
use std::path::{Path, PathBuf};

use cockpit_domain::{
    AppConfig, DashboardSections, DisplayProfile, Event, NotesConfig, Shortcut, ThemeConfig,
    UiConfig, WidgetPosition, WindowConfig,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid TOML config in {path}: {source}")]
    InvalidToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("invalid window position: {0}")]
    InvalidPosition(String),
    #[error("invalid display profile: {0}")]
    InvalidDisplayProfile(String),
    #[error("refresh_interval_seconds must be at least 1")]
    InvalidRefreshInterval,
}

#[derive(Debug, Default)]
pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(path: Option<&Path>) -> Result<AppConfig, ConfigError> {
        let path = match path {
            Some(path) => path.to_path_buf(),
            None => default_config_path()
                .unwrap_or_else(|| PathBuf::from("examples/config.example.toml")),
        };

        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Result<AppConfig, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let raw =
            toml::from_str::<RawConfig>(&content).map_err(|source| ConfigError::InvalidToml {
                path: path.to_path_buf(),
                source,
            })?;

        raw.into_domain()
    }
}

fn default_config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let user_config = PathBuf::from(home).join(".config/desktop-cockpit/config.toml");
    if user_config.exists() {
        Some(user_config)
    } else {
        let example = PathBuf::from("examples/config.example.toml");
        example.exists().then_some(example)
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    app: Option<RawApp>,
    window: Option<RawWindow>,
    theme: Option<RawTheme>,
    ui: Option<RawUi>,
    sections: Option<RawSections>,
    notes: Option<RawNotes>,
    #[serde(default)]
    events: Vec<RawEvent>,
    #[serde(default)]
    shortcuts: Vec<RawShortcut>,
}

impl RawConfig {
    fn into_domain(self) -> Result<AppConfig, ConfigError> {
        let mut config = AppConfig::default();

        if let Some(app) = self.app {
            if let Some(name) = app.name {
                config.name = name;
            }
            if let Some(refresh_interval_seconds) = app.refresh_interval_seconds {
                if refresh_interval_seconds == 0 {
                    return Err(ConfigError::InvalidRefreshInterval);
                }
                config.refresh_interval_seconds = refresh_interval_seconds;
            }
        }

        if let Some(window) = self.window {
            let defaults = WindowConfig::default();
            config.window = WindowConfig {
                width: window.width.unwrap_or(defaults.width),
                height: window.height.unwrap_or(defaults.height),
                monitor: window.monitor.unwrap_or(defaults.monitor),
                position: match window.position {
                    Some(position) => parse_position(&position)?,
                    None => defaults.position,
                },
                margin_top: window.margin_top.unwrap_or(defaults.margin_top),
                margin_right: window.margin_right.unwrap_or(defaults.margin_right),
                opacity: window.opacity.unwrap_or(defaults.opacity),
                always_on_top: window.always_on_top.unwrap_or(defaults.always_on_top),
            };
        }

        if let Some(theme) = self.theme {
            let defaults = ThemeConfig::default();
            config.theme = ThemeConfig {
                font_family: theme.font_family.unwrap_or(defaults.font_family),
                font_size: theme.font_size.unwrap_or(defaults.font_size),
                border_radius: theme.border_radius.unwrap_or(defaults.border_radius),
                padding: theme.padding.unwrap_or(defaults.padding),
            };
        }

        if let Some(ui) = self.ui {
            let defaults = UiConfig::default();
            config.ui = UiConfig {
                display_profile: match ui.display_profile {
                    Some(display_profile) => parse_display_profile(&display_profile)?,
                    None => defaults.display_profile,
                },
                show_sound_test_button: ui
                    .show_sound_test_button
                    .unwrap_or(defaults.show_sound_test_button),
            };
        }

        if let Some(sections) = self.sections {
            let defaults = DashboardSections::default();
            config.sections = DashboardSections {
                show_clock: sections.show_clock.unwrap_or(defaults.show_clock),
                show_events: sections.show_events.unwrap_or(defaults.show_events),
                show_tasks: sections.show_tasks.unwrap_or(defaults.show_tasks),
                show_system: sections.show_system.unwrap_or(defaults.show_system),
                show_shortcuts: sections.show_shortcuts.unwrap_or(defaults.show_shortcuts),
            };
        }

        if let Some(notes) = self.notes {
            config.notes = NotesConfig {
                daily_file: notes.daily_file,
            };
        }

        config.events = self
            .events
            .into_iter()
            .map(|event| Event {
                time: event.time,
                title: event.title,
                description: event.description,
            })
            .collect();

        config.shortcuts = self
            .shortcuts
            .into_iter()
            .map(|shortcut| Shortcut {
                label: shortcut.label,
                command: shortcut.command,
            })
            .collect();

        Ok(config)
    }
}

fn parse_position(position: &str) -> Result<WidgetPosition, ConfigError> {
    match position {
        "top-left" => Ok(WidgetPosition::TopLeft),
        "top-right" => Ok(WidgetPosition::TopRight),
        "bottom-left" => Ok(WidgetPosition::BottomLeft),
        "bottom-right" => Ok(WidgetPosition::BottomRight),
        other => Err(ConfigError::InvalidPosition(other.to_string())),
    }
}

fn parse_display_profile(display_profile: &str) -> Result<DisplayProfile, ConfigError> {
    match display_profile {
        "plain" => Ok(DisplayProfile::Plain),
        "test-all-features" => Ok(DisplayProfile::TestAllFeatures),
        other => Err(ConfigError::InvalidDisplayProfile(other.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct RawApp {
    name: Option<String>,
    refresh_interval_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RawWindow {
    width: Option<u32>,
    height: Option<u32>,
    monitor: Option<u32>,
    position: Option<String>,
    margin_top: Option<u32>,
    margin_right: Option<u32>,
    opacity: Option<f32>,
    always_on_top: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawTheme {
    font_family: Option<String>,
    font_size: Option<u16>,
    border_radius: Option<u16>,
    padding: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct RawUi {
    display_profile: Option<String>,
    show_sound_test_button: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawSections {
    show_clock: Option<bool>,
    show_events: Option<bool>,
    show_tasks: Option<bool>,
    show_system: Option<bool>,
    show_shortcuts: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawNotes {
    daily_file: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawEvent {
    time: String,
    title: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawShortcut {
    label: String,
    command: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_valid_config() {
        let path = write_temp_config(
            "valid",
            r#"
[app]
name = "Test Cockpit"
refresh_interval_seconds = 10

[ui]
display_profile = "test-all-features"
show_sound_test_button = true

[[events]]
time = "09:00"
title = "Planning"
"#,
        );

        let config = ConfigLoader::load_from_path(&path).unwrap();

        assert_eq!(config.name, "Test Cockpit");
        assert_eq!(config.refresh_interval_seconds, 10);
        assert_eq!(config.ui.display_profile, DisplayProfile::TestAllFeatures);
        assert!(config.ui.show_sound_test_button);
        assert_eq!(config.events[0].title, "Planning");
    }

    #[test]
    fn rejects_invalid_config() {
        let path = write_temp_config("invalid", "not = [valid");

        let error = ConfigLoader::load_from_path(&path).unwrap_err();

        assert!(matches!(error, ConfigError::InvalidToml { .. }));
    }

    #[test]
    fn applies_defaults() {
        let path = write_temp_config("defaults", "");

        let config = ConfigLoader::load_from_path(&path).unwrap();

        assert_eq!(config.name, "Desktop Cockpit");
        assert_eq!(config.window.width, 360);
        assert_eq!(config.refresh_interval_seconds, 5);
        assert!(config.sections.show_clock);
    }

    fn write_temp_config(name: &str, content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "desktop-cockpit-{name}-{}-{}.toml",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }
}
