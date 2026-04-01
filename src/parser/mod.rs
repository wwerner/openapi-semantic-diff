// src/parser/mod.rs — Parser trait, version detection, and entry point

pub mod openapi30;
pub mod openapi31;

use crate::model::{InternalSpec, OsdError};
use std::path::Path;

/// Detect OpenAPI version from raw text by looking for the `openapi:` field.
/// Returns the version string (e.g. "3.0.3", "3.1.0").
fn detect_version(content: &str) -> Result<String, OsdError> {
    // Try JSON first
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(v) = val.get("openapi").and_then(|v| v.as_str()) {
            return Ok(v.to_string());
        }
        return Err(OsdError::UnsupportedVersion(
            "missing 'openapi' field in JSON".to_string(),
        ));
    }

    // Try YAML
    if let Ok(val) = serde_yml::from_str::<serde_json::Value>(content) {
        if let Some(v) = val.get("openapi").and_then(|v| v.as_str()) {
            return Ok(v.to_string());
        }
        // openapi version might be parsed as a number (e.g. 3.0 -> f64)
        if let Some(v) = val.get("openapi").and_then(|v| v.as_f64()) {
            return Ok(format!("{v}"));
        }
        return Err(OsdError::UnsupportedVersion(
            "missing 'openapi' field in YAML".to_string(),
        ));
    }

    Err(OsdError::Other(
        "failed to parse input as JSON or YAML".to_string(),
    ))
}

/// Parse an OpenAPI spec from a file path. Auto-detects format (JSON/YAML) and
/// OpenAPI version (3.0.x / 3.1.x), then delegates to the appropriate parser.
pub fn parse_file(path: &Path) -> Result<InternalSpec, OsdError> {
    let content = std::fs::read_to_string(path)?;
    parse_str(&content, Some(path))
}

/// Parse an OpenAPI spec from a string. `source_path` is optional and used for
/// resolving relative `$ref`s.
pub fn parse_str(content: &str, source_path: Option<&Path>) -> Result<InternalSpec, OsdError> {
    let version = detect_version(content)?;

    if version.starts_with("3.0") {
        openapi30::parse(content, source_path)
    } else if version.starts_with("3.1") {
        openapi31::parse(content, source_path)
    } else {
        Err(OsdError::UnsupportedVersion(version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_version_yaml_30() {
        let yaml = "openapi: 3.0.3\ninfo:\n  title: Test\n  version: '1.0'\npaths: {}";
        assert_eq!(detect_version(yaml).unwrap(), "3.0.3");
    }

    #[test]
    fn detect_version_yaml_31() {
        let yaml = "openapi: '3.1.0'\ninfo:\n  title: Test\n  version: '1.0'\npaths: {}";
        assert_eq!(detect_version(yaml).unwrap(), "3.1.0");
    }

    #[test]
    fn detect_version_json_30() {
        let json =
            r#"{"openapi": "3.0.2", "info": {"title": "Test", "version": "1.0"}, "paths": {}}"#;
        assert_eq!(detect_version(json).unwrap(), "3.0.2");
    }

    #[test]
    fn detect_version_missing() {
        let yaml = "info:\n  title: Test";
        assert!(detect_version(yaml).is_err());
    }

    #[test]
    fn detect_version_unsupported() {
        let yaml = "openapi: '2.0'\ninfo:\n  title: Test\n  version: '1.0'";
        // Version detection itself succeeds; routing would fail
        let v = detect_version(yaml).unwrap();
        assert_eq!(v, "2.0");
    }
}
