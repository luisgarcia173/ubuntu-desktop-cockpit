# Architecture

Desktop Cockpit is organized as a Cargo workspace with thin crates by responsibility.

- `cockpit-domain`: pure application models.
- `cockpit-config`: TOML loading, defaults, and validation.
- `cockpit-notes`: local Markdown task parsing.
- `cockpit-calendar`: local event loading from configuration.
- `cockpit-system`: operating system metrics.
- `cockpit-core`: use cases and provider traits.
- `cockpit-ui-gtk`: GTK presentation.
- `cockpit-app`: composition root and executable entry point.

The UI crate depends inward on domain/core concepts. Domain does not depend on GTK, filesystem, TOML, or system integrations.
