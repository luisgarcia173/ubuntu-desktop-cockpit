#[derive(Debug, Clone, PartialEq)]
pub struct Dashboard {
    pub app_name: String,
    pub sections: DashboardSections,
    pub clock: ClockInfo,
    pub events: Vec<Event>,
    pub tasks: Vec<Task>,
    pub system: SystemMetric,
    pub shortcuts: Vec<Shortcut>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardSection {
    Clock,
    Events,
    Tasks,
    System,
    Shortcuts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardSections {
    pub show_clock: bool,
    pub show_events: bool,
    pub show_tasks: bool,
    pub show_system: bool,
    pub show_shortcuts: bool,
}

impl Default for DashboardSections {
    fn default() -> Self {
        Self {
            show_clock: true,
            show_events: true,
            show_tasks: true,
            show_system: true,
            show_shortcuts: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockInfo {
    pub time: String,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub time: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemMetric {
    pub cpu_usage_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_usage_percent: f32,
}

impl Default for SystemMetric {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_total_bytes: 0,
            memory_used_bytes: 0,
            memory_usage_percent: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum WidgetPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeConfig {
    pub font_family: String,
    pub font_size: u16,
    pub border_radius: u16,
    pub padding: u16,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 13,
            border_radius: 16,
            padding: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub monitor: u32,
    pub position: WidgetPosition,
    pub margin_top: u32,
    pub margin_right: u32,
    pub opacity: f32,
    pub always_on_top: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 360,
            height: 520,
            monitor: 1,
            position: WidgetPosition::TopRight,
            margin_top: 32,
            margin_right: 24,
            opacity: 0.92,
            always_on_top: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesConfig {
    pub daily_file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DisplayProfile {
    #[default]
    Plain,
    TestAllFeatures,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiConfig {
    pub display_profile: DisplayProfile,
    pub show_sound_test_button: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            display_profile: DisplayProfile::Plain,
            show_sound_test_button: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub name: String,
    pub refresh_interval_seconds: u64,
    pub window: WindowConfig,
    pub theme: ThemeConfig,
    pub ui: UiConfig,
    pub sections: DashboardSections,
    pub notes: NotesConfig,
    pub events: Vec<Event>,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Desktop Cockpit".to_string(),
            refresh_interval_seconds: 5,
            window: WindowConfig::default(),
            theme: ThemeConfig::default(),
            ui: UiConfig::default(),
            sections: DashboardSections::default(),
            notes: NotesConfig { daily_file: None },
            events: Vec::new(),
            shortcuts: Vec::new(),
        }
    }
}
