# Decisions

## Native Rust And GTK

Desktop Cockpit uses Rust and GTK instead of Electron, Tauri, WebView, or browser-based UI. The goal is low memory use and native Linux desktop integration.

## Local-First V1

The first version reads local TOML and Markdown files only. External account integrations are deferred.

## Optional GTK Feature During Early Development

The GTK UI is behind the `gtk-ui` feature so the domain, config, notes, calendar, system, and core crates can be checked and tested even on machines without GTK development packages installed.
