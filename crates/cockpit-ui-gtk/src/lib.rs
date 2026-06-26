use cockpit_domain::{AppConfig, Dashboard};
#[cfg(feature = "gtk-ui")]
use cockpit_domain::DisplayProfile;
#[cfg(feature = "gtk-ui")]
use gtk4::prelude::*;
#[cfg(feature = "gtk-ui")]
use std::path::{Path, PathBuf};
#[cfg(feature = "gtk-ui")]
use std::{cell::Cell, process::Stdio, rc::Rc};

#[cfg(feature = "gtk-ui")]
pub fn run_dashboard(config: &AppConfig, dashboard: Dashboard) {
    use gtk4 as gtk;

    let app = gtk::Application::builder()
        .application_id("dev.desktop_cockpit.app")
        .build();
    let config = config.clone();

    app.connect_activate(move |app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_str!("../assets/style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let root = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        root.add_css_class("cockpit-panel");
        root.set_margin_top(config.theme.padding.into());
        root.set_margin_bottom(config.theme.padding.into());
        root.set_margin_start(config.theme.padding.into());
        root.set_margin_end(config.theme.padding.into());

        let side_rail = gtk::Box::new(gtk::Orientation::Vertical, 6);
        side_rail.add_css_class("side-rail");

        let side_title = gtk::Label::new(Some("D\nE\nS\nK\nT\nO\nP\n\nC\nO\nC\nK\nP\nI\nT"));
        side_title.add_css_class("side-title");
        side_title.set_xalign(0.5);
        side_title.set_vexpand(true);
        side_rail.append(&side_title);

        let reload_button = gtk::Button::with_label("⟳");
        reload_button.add_css_class("reload-button");
        reload_button.set_tooltip_text(Some("Atualizar cockpit agora"));
        let app_for_reload = app.clone();
        reload_button.connect_clicked(move |_| match reload_cockpit_process() {
            Ok(_) => app_for_reload.quit(),
            Err(error) => eprintln!("failed to reload cockpit: {error}"),
        });
        side_rail.append(&reload_button);

        let close_button = gtk::Button::with_label("⏻");
        close_button.add_css_class("close-button");
        close_button.set_tooltip_text(Some("Encerrar Desktop Cockpit"));
        let app_for_close = app.clone();
        close_button.connect_clicked(move |_| app_for_close.quit());
        side_rail.append(&close_button);

//         let side_version = gtk::Label::new(Some("V\n.\n1\n.\n0\n.\n0"));
//         side_version.add_css_class("side-version");
//         side_version.set_xalign(0.5);
//         side_rail.append(&side_version);

        root.append(&side_rail);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        content.add_css_class("panel-content");
        content.set_hexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
        header.add_css_class("panel-header");
        header.set_hexpand(true);

        if dashboard.sections.show_clock {
            let clock_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
            clock_box.add_css_class("clock-box");
            clock_box.set_hexpand(true);

            let time = gtk::Label::new(Some(&dashboard.clock.time));
            time.add_css_class("clock-time");
            time.set_xalign(0.5);
            clock_box.append(&time);

            let date = gtk::Label::new(Some(&dashboard.clock.date));
            date.add_css_class("clock-date");
            date.set_xalign(0.5);
            clock_box.append(&date);

            header.append(&clock_box);
        }

        content.append(&header);

        let cockpit_body = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        cockpit_body.add_css_class("cockpit-body");
        cockpit_body.set_hexpand(true);

        let main_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        main_panel.add_css_class("main-panel");
        main_panel.set_hexpand(true);

        if dashboard.sections.show_events {
            append_heading(&main_panel, "Proximos eventos");
            for event in &dashboard.events {
                append_kv_row(&main_panel, &event.time, &event.title);
            }
        }

        if dashboard.sections.show_system {
            append_recent_projects_section(&main_panel);
        }

        cockpit_body.append(&main_panel);

        content.append(&cockpit_body);

        if dashboard.sections.show_shortcuts {
            append_shortcuts_panel(&content, &dashboard);
        }

        append_pomodoro_panel(&content, &config);

        append_environment_status_panel(&content);

        root.append(&content);

        let window_width = effective_window_width(&root, config.window.width);

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title(&config.name)
            .default_width(window_width)
            .default_height(config.window.height as i32)
            .decorated(false)
            .child(&root)
            .build();

        window.set_opacity(config.window.opacity as f64);
        window.present();
    });

    app.run_with_args(&["cockpit-app"]);
}

#[cfg(not(feature = "gtk-ui"))]
pub fn run_dashboard(_config: &AppConfig, dashboard: Dashboard) {
    println!("{}", render_text_dashboard(&dashboard));
}

pub fn render_text_dashboard(dashboard: &Dashboard) -> String {
    let mut output = String::new();

    if dashboard.sections.show_clock {
        output.push_str(&format!(
            "{}\n{}\n\n",
            dashboard.clock.time, dashboard.clock.date
        ));
    }
    if dashboard.sections.show_events {
        output.push_str("Proximos eventos\n");
        for event in &dashboard.events {
            output.push_str(&format!("{}  {}\n", event.time, event.title));
        }
        output.push('\n');
    }
    if dashboard.sections.show_tasks {
        output.push_str("Top 3\n");
        for task in &dashboard.tasks {
            output.push_str(&format!("[] {}\n", task.title));
        }
        output.push('\n');
    }
    if dashboard.sections.show_system {
        output.push_str("Sistema\n");
        output.push_str(&format!(
            "CPU  {:02.0}%\n",
            dashboard.system.cpu_usage_percent
        ));
        output.push_str(&format!(
            "RAM  {:02.0}%\n\n",
            dashboard.system.memory_usage_percent
        ));
    }
    if dashboard.sections.show_shortcuts {
        output.push_str("Atalhos\n");
        for shortcut in &dashboard.shortcuts {
            output.push_str(&format!("[{}] ", shortcut.label));
        }
        output.push('\n');
    }

    output
}

#[cfg(feature = "gtk-ui")]
fn append_heading(root: &gtk4::Box, text: &str) {
    let heading = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    heading.add_css_class("section-heading-row");

    let marker = gtk4::Label::new(Some("//"));
    marker.add_css_class("section-marker");
    marker.set_xalign(0.0);
    heading.append(&marker);

    let label = gtk4::Label::new(Some(text));
    label.add_css_class("section-heading");
    label.set_xalign(0.0);
    label.set_hexpand(true);
    heading.append(&label);

    root.append(&heading);
}

#[cfg(feature = "gtk-ui")]
fn append_kv_row(root: &gtk4::Box, key: &str, value: &str) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    row.add_css_class("data-row");

    let key = gtk4::Label::new(Some(key));
    key.add_css_class("row-key");
    key.set_xalign(0.0);
    row.append(&key);

    let value = gtk4::Label::new(Some(value));
    value.add_css_class("row-value");
    value.set_xalign(0.0);
    value.set_hexpand(true);
    row.append(&value);

    root.append(&row);
}

#[cfg(feature = "gtk-ui")]
fn append_recent_projects_section(root: &gtk4::Box) {
    append_heading(root, "Ultimos projetos");

    let projects = recent_intellij_projects(3);
    if projects.is_empty() {
        append_kv_row(root, "◇", "Nenhum historico local");
        return;
    }

    for (index, project) in projects.iter().enumerate() {
        append_kv_row(root, &format!("{:02}", index + 1), project);
    }
}

#[cfg(feature = "gtk-ui")]
fn append_shortcuts_panel(root: &gtk4::Box, dashboard: &Dashboard) {
    let panel = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    panel.add_css_class("extra-panel");

    append_heading(&panel, "Atalhos");

    let shortcuts = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    shortcuts.add_css_class("shortcuts");
    for shortcut in &dashboard.shortcuts {
        let button = gtk4::Button::with_label(&format!(
            "{} {}",
            shortcut_icon(&shortcut.label),
            shortcut.label
        ));
        button.add_css_class("shortcut-button");
        let command = shortcut.command.clone();
        let label = shortcut.label.clone();
        button.connect_clicked(move |_| {
            if let Err(error) = spawn_shortcut_command(&label, &command) {
                eprintln!("failed to run shortcut `{label}` with `{command}`: {error}");
            }
        });
        shortcuts.append(&button);
    }
    panel.append(&shortcuts);

    root.append(&panel);
}

#[cfg(feature = "gtk-ui")]
fn append_environment_status_panel(root: &gtk4::Box) {
    let panel = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    panel.add_css_class("status-panel");

    append_heading(&panel, "Status ambiente");

    let running = running_containers();
    if running.is_empty() {
        let empty = gtk4::Label::new(Some(
            "Nenhum container Docker/Podman em execucao no momento.\nSuba seus servicos e o cockpit atualiza automaticamente.",
        ));
        empty.add_css_class("status-empty");
        empty.set_xalign(0.0);
        panel.append(&empty);
        root.append(&panel);
        return;
    }

    let status_json = running_containers_json_like(&running);
    let status_text = gtk4::Label::new(Some(&status_json));
    status_text.add_css_class("status-json");
    status_text.set_xalign(0.0);
    status_text.set_selectable(true);
    panel.append(&status_text);

    root.append(&panel);
}

#[cfg(feature = "gtk-ui")]
fn append_pomodoro_panel(root: &gtk4::Box, config: &AppConfig) {
    let panel = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    panel.add_css_class("pomodoro-panel");
    append_pomodoro_section(&panel, should_show_sound_test_button(config));
    root.append(&panel);
}

#[cfg(feature = "gtk-ui")]
fn append_pomodoro_section(root: &gtk4::Box, show_sound_test_button: bool) {
    let controls_panel = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    controls_panel.add_css_class("pomodoro-controls-panel");
    controls_panel.set_hexpand(true);

    append_heading(&controls_panel, "Pomodoro");

    let timer_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    timer_row.add_css_class("pomodoro-timer-strip");
    timer_row.set_halign(gtk4::Align::Fill);
    timer_row.set_hexpand(true);

    let timer_label = gtk4::Label::new(Some("00:00"));
    timer_label.add_css_class("pomodoro-time");
    timer_label.set_xalign(0.0);
    timer_label.set_hexpand(true);
    timer_row.append(&timer_label);

    let pause_button = gtk4::Button::with_label("▶");
    pause_button.add_css_class("pomodoro-switch");
    pause_button.set_tooltip_text(Some("Pausar/retomar"));
    pause_button.set_sensitive(false);

    let reset_button = gtk4::Button::with_label("↺");
    reset_button.add_css_class("pomodoro-reset");
    reset_button.set_tooltip_text(Some("Resetar timer"));
    reset_button.set_sensitive(false);

    let timer_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    timer_actions.add_css_class("pomodoro-timer-actions");
    timer_actions.set_halign(gtk4::Align::End);
    timer_actions.append(&pause_button);
    timer_actions.append(&reset_button);
    timer_row.append(&timer_actions);

    let status_label = gtk4::Label::new(Some("Aguardando protocolo"));
    status_label.add_css_class("pomodoro-status");
    status_label.set_xalign(0.0);
    controls_panel.append(&status_label);
    controls_panel.append(&timer_row);

    let controls = gtk4::FlowBox::new();
    controls.add_css_class("pomodoro-controls");
    controls.set_selection_mode(gtk4::SelectionMode::None);
    controls.set_min_children_per_line(2);
    controls.set_max_children_per_line(4);
    controls.set_halign(gtk4::Align::Fill);
    controls.set_hexpand(true);

    let remaining_seconds = Rc::new(Cell::new(0_u32));
    let current_mode = Rc::new(std::cell::RefCell::new(None));
    let source_id = Rc::new(std::cell::RefCell::new(None));
    let mode_buttons = Rc::new(std::cell::RefCell::new(Vec::new()));
    let runtime = PomodoroRuntime {
        timer_label: timer_label.clone(),
        status_label: status_label.clone(),
        pause_button: pause_button.clone(),
        reset_button: reset_button.clone(),
        remaining_seconds,
        current_mode,
        source_id,
        mode_buttons: mode_buttons.clone(),
    };

    for mode in [
        TimerMode::new("Aula", "◈", 50),
        TimerMode::new("Desenv", "◈", 60),
        TimerMode::new("Pomodoro", "◈", 30),
    ] {
        let button = gtk4::Button::with_label(&format!("{} {}", mode.icon, mode.label));
        button.add_css_class("pomodoro-button");
        mode_buttons
            .borrow_mut()
            .push((mode.label.to_string(), button.clone()));

        let runtime = runtime.clone();

        button.connect_clicked(move |_| {
            start_pomodoro_timer(mode, runtime.clone());
        });

        controls.insert(&button, -1);
    }

    if show_sound_test_button {
        let sound_test_button = gtk4::Button::with_label("◈ Som");
        sound_test_button.add_css_class("pomodoro-button");
        sound_test_button.set_tooltip_text(Some("Testar aviso sonoro"));
        sound_test_button.connect_clicked(|_| play_timer_sound());
        controls.insert(&sound_test_button, -1);
    }

    controls_panel.append(&controls);
    root.append(&controls_panel);

    let runtime_for_pause = runtime.clone();
    pause_button.connect_clicked(move |_| {
        toggle_pomodoro_pause(runtime_for_pause.clone());
    });

    let runtime_for_reset = runtime;
    reset_button.connect_clicked(move |_| {
        reset_pomodoro_timer(runtime_for_reset.clone());
    });
}

#[cfg(feature = "gtk-ui")]
fn start_pomodoro_timer(mode: TimerMode, runtime: PomodoroRuntime) {
    if let Some(source_id) = runtime.source_id.borrow_mut().take() {
        source_id.remove();
    }

    let duration_seconds = mode.minutes * 60;
    runtime.remaining_seconds.set(duration_seconds);
    *runtime.current_mode.borrow_mut() = Some(mode);
    runtime
        .timer_label
        .set_label(&format_timer(duration_seconds));
    runtime
        .status_label
        .set_label(&format!("{} em andamento", mode.label));
    runtime.pause_button.set_label("⏸");
    runtime.pause_button.set_sensitive(true);
    runtime.reset_button.set_sensitive(true);
    update_mode_button_state(&runtime, Some(mode.label));

    schedule_pomodoro_tick(mode, runtime);
}

#[cfg(feature = "gtk-ui")]
fn schedule_pomodoro_tick(mode: TimerMode, runtime: PomodoroRuntime) {
    let source_handle = runtime.source_id.clone();
    let timer_label = runtime.timer_label.clone();
    let status_label = runtime.status_label.clone();
    let pause_button = runtime.pause_button.clone();
    let reset_button = runtime.reset_button.clone();
    let remaining_seconds = runtime.remaining_seconds.clone();

    let id = glib::timeout_add_seconds_local(1, move || {
        let remaining = remaining_seconds.get().saturating_sub(1);
        remaining_seconds.set(remaining);
        timer_label.set_label(&format_timer(remaining));

        if remaining == 0 {
            status_label.set_label(&format!("{} finalizado", mode.label));
            pause_button.set_label("▶");
            pause_button.set_sensitive(false);
            reset_button.set_sensitive(false);
            play_timer_sound();
            *source_handle.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    *runtime.source_id.borrow_mut() = Some(id);
}

#[cfg(feature = "gtk-ui")]
fn toggle_pomodoro_pause(runtime: PomodoroRuntime) {
    if runtime.remaining_seconds.get() == 0 {
        return;
    }

    if let Some(source_id) = runtime.source_id.borrow_mut().take() {
        source_id.remove();
        runtime.pause_button.set_label("▶");
        if let Some(mode) = runtime.current_mode.borrow().as_ref() {
            runtime
                .status_label
                .set_label(&format!("{} pausado", mode.label));
        }
        return;
    }

    let Some(mode) = *runtime.current_mode.borrow() else {
        return;
    };

    runtime.pause_button.set_label("⏸");
    runtime
        .status_label
        .set_label(&format!("{} em andamento", mode.label));
    schedule_pomodoro_tick(mode, runtime);
}

#[cfg(feature = "gtk-ui")]
fn reset_pomodoro_timer(runtime: PomodoroRuntime) {
    if let Some(source_id) = runtime.source_id.borrow_mut().take() {
        source_id.remove();
    }

    runtime.remaining_seconds.set(0);
    *runtime.current_mode.borrow_mut() = None;
    runtime.timer_label.set_label("00:00");
    runtime.status_label.set_label("Aguardando protocolo");
    runtime.pause_button.set_label("▶");
    runtime.pause_button.set_sensitive(false);
    runtime.reset_button.set_sensitive(false);
    update_mode_button_state(&runtime, None);
}

#[cfg(feature = "gtk-ui")]
fn update_mode_button_state(runtime: &PomodoroRuntime, active_label: Option<&'static str>) {
    for (label, button) in runtime.mode_buttons.borrow().iter() {
        let is_active = active_label.is_some_and(|active| active == label);
        if is_active {
            button.add_css_class("pomodoro-button-active");
        } else {
            button.remove_css_class("pomodoro-button-active");
        }
    }
}

#[cfg(feature = "gtk-ui")]
fn should_show_sound_test_button(config: &AppConfig) -> bool {
    matches!(config.ui.display_profile, DisplayProfile::TestAllFeatures)
        || config.ui.show_sound_test_button
}

#[cfg(feature = "gtk-ui")]
#[derive(Clone, Copy)]
struct TimerMode {
    label: &'static str,
    icon: &'static str,
    minutes: u32,
}

#[cfg(feature = "gtk-ui")]
#[derive(Clone)]
struct PomodoroRuntime {
    timer_label: gtk4::Label,
    status_label: gtk4::Label,
    pause_button: gtk4::Button,
    reset_button: gtk4::Button,
    remaining_seconds: Rc<Cell<u32>>,
    current_mode: Rc<std::cell::RefCell<Option<TimerMode>>>,
    source_id: Rc<std::cell::RefCell<Option<glib::SourceId>>>,
    mode_buttons: Rc<std::cell::RefCell<Vec<(String, gtk4::Button)>>>,
}

#[cfg(feature = "gtk-ui")]
impl TimerMode {
    const fn new(label: &'static str, icon: &'static str, minutes: u32) -> Self {
        Self {
            label,
            icon,
            minutes,
        }
    }
}

#[cfg(feature = "gtk-ui")]
fn shortcut_icon(_label: &str) -> &'static str {
    "◈"
}

#[cfg(feature = "gtk-ui")]
fn spawn_shortcut_command(label: &str, command: &str) -> std::io::Result<std::process::Child> {
    let resolved = resolve_shortcut_command(label, command);
    std::process::Command::new("sh")
        .args(["-lc", &resolved])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[cfg(feature = "gtk-ui")]
fn resolve_shortcut_command(label: &str, command: &str) -> String {
    if label.eq_ignore_ascii_case("intellij") && command.trim() == "idea" {
        return "$HOME/.local/share/JetBrains/Toolbox/scripts/idea || idea || intellij-idea-ultimate || intellij-idea-community || flatpak run com.jetbrains.IntelliJ-IDEA-Ultimate || flatpak run com.jetbrains.IntelliJ-IDEA-Community".to_string();
    }

    if label.eq_ignore_ascii_case("podman") {
        return "podman-desktop || flatpak run io.podman_desktop.PodmanDesktop".to_string();
    }

    command.to_string()
}

#[cfg(feature = "gtk-ui")]
fn reload_cockpit_process() -> std::io::Result<std::process::Child> {
    let executable = std::env::current_exe()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    std::process::Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[cfg(feature = "gtk-ui")]
fn effective_window_width(root: &gtk4::Box, config_width: u32) -> i32 {
    let (_, natural, _, _) = root.measure(gtk4::Orientation::Horizontal, -1);
    let fallback = (config_width as i32).max(1);
    let preferred = if natural > 0 { natural } else { fallback };
    preferred.min(900)
}


#[cfg(feature = "gtk-ui")]
#[derive(Debug, Clone)]
struct ContainerInfo {
    name: String,
    engine: &'static str,
}

#[cfg(feature = "gtk-ui")]
fn running_containers() -> Vec<ContainerInfo> {
    let mut containers = containers_from_engine("docker");
    containers.extend(containers_from_engine("podman"));
    containers.sort_by(|a, b| a.engine.cmp(b.engine).then_with(|| a.name.cmp(&b.name)));
    containers.dedup_by(|a, b| a.engine == b.engine && a.name == b.name);
    containers
}

#[cfg(feature = "gtk-ui")]
fn containers_from_engine(engine: &'static str) -> Vec<ContainerInfo> {
    let output = std::process::Command::new(engine)
        .args(["ps", "--format", "{{.Names}}"])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let name = line.trim();
            if name.is_empty() {
                return None;
            }

            Some(ContainerInfo {
                name: name.to_ascii_lowercase(),
                engine,
            })
        })
        .collect()
}

#[cfg(feature = "gtk-ui")]
fn running_containers_json_like(containers: &[ContainerInfo]) -> String {
    let mut lines = Vec::with_capacity(containers.len() + 2);
    lines.push("{".to_string());

    for (index, container) in containers.iter().enumerate() {
        let comma = if index + 1 == containers.len() {
            ""
        } else {
            ","
        };
        lines.push(format!(
            "  \"{}@{}\": \"UP\"{comma}",
            container.name, container.engine
        ));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

#[cfg(feature = "gtk-ui")]
fn recent_intellij_projects(limit: usize) -> Vec<String> {
    let Some(path) = latest_intellij_recent_projects_file() else {
        return Vec::new();
    };

    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut projects = Vec::new();
    for line in content.lines().filter(|line| line.contains("<entry key=")) {
        let Some(path) = extract_xml_attribute(line, "key") else {
            continue;
        };
        let path = expand_user_home(&path);
        let Some(name) = Path::new(&path).file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !projects.iter().any(|project| project == name) {
            projects.push(name.to_string());
        }
    }

    projects.into_iter().rev().take(limit).collect()
}

#[cfg(feature = "gtk-ui")]
fn latest_intellij_recent_projects_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let jetbrains_dir = PathBuf::from(home).join(".config/JetBrains");
    let entries = std::fs::read_dir(jetbrains_dir).ok()?;

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("options/recentProjects.xml"))
        .filter(|path| path.exists())
        .max()
}

#[cfg(feature = "gtk-ui")]
fn extract_xml_attribute(line: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = line.find(&pattern)? + pattern.len();
    let end = line[start..].find('"')?;
    Some(line[start..start + end].to_string())
}

#[cfg(feature = "gtk-ui")]
fn expand_user_home(path: &str) -> String {
    let Some(home) = std::env::var("HOME").ok() else {
        return path.to_string();
    };
    path.replace("$USER_HOME$", &home)
}

#[cfg(feature = "gtk-ui")]
fn format_timer(total_seconds: u32) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

#[cfg(feature = "gtk-ui")]
fn play_timer_sound() {
    std::thread::spawn(|| {
        let sound = "/usr/share/sounds/freedesktop/stereo/message-new-instant.oga";
        if std::path::Path::new(sound).exists() {
            let mut played = false;
            for _ in 0..3 {
                if std::process::Command::new("paplay")
                    .args(["--volume=90000", sound])
                    .status()
                    .is_ok()
                {
                    played = true;
                    std::thread::sleep(std::time::Duration::from_millis(140));
                }
            }

            if played {
                return;
            }
        }

        let mut played = false;
        for _ in 0..3 {
            if std::process::Command::new("canberra-gtk-play")
                .args(["-i", "message-new-instant"])
                .status()
                .is_ok()
            {
                played = true;
                std::thread::sleep(std::time::Duration::from_millis(140));
            }
        }

        if !played {
            eprintln!("failed to play pomodoro sound");
        }
    });
}
