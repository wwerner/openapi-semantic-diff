# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2025-04-01

### Added

- Initial release.
- Compare OpenAPI 3.0.x and 3.1.x specifications (YAML and JSON).
- Three severity tiers: breaking, deprecated, additive.
- Four built-in output formats: text, markdown, json, html.
- Custom Tera template support.
- Pluggable `x-*` extension processor system.
- Built-in `x-extensible-enum` processor.
- `diff` subcommand for comparing specs.
- `check` subcommand for CI gating on breaking changes.
- `templates` subcommand for exporting built-in templates.
- Severity filtering via `--min-severity`.
