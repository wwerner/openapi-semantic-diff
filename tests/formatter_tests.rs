// tests/formatter_tests.rs — Snapshot tests for formatter output using insta

use openapi_semantic_diff::comparator::compare;
use openapi_semantic_diff::formatter::{format_report, OutputFormat};
use openapi_semantic_diff::parser;
use std::path::Path;

fn load(name: &str) -> openapi_semantic_diff::model::InternalSpec {
    let path = Path::new("tests/fixtures").join(name);
    parser::parse_file(&path).unwrap_or_else(|e| panic!("failed to parse {name}: {e}"))
}

// ---------------------------------------------------------------------------
// Breaking changes — all formats
// ---------------------------------------------------------------------------

#[test]
fn snapshot_breaking_text() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/breaking.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Text).unwrap();
    insta::assert_snapshot!("breaking_text", output);
}

#[test]
fn snapshot_breaking_markdown() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/breaking.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Markdown).unwrap();
    insta::assert_snapshot!("breaking_markdown", output);
}

#[test]
fn snapshot_breaking_json() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/breaking.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Json).unwrap();
    insta::assert_snapshot!("breaking_json", output);
}

#[test]
fn snapshot_breaking_html() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/breaking.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Html).unwrap();
    insta::assert_snapshot!("breaking_html", output);
}

// ---------------------------------------------------------------------------
// Additive changes — text
// ---------------------------------------------------------------------------

#[test]
fn snapshot_additive_text() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/additive.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Text).unwrap();
    insta::assert_snapshot!("additive_text", output);
}

// ---------------------------------------------------------------------------
// Deprecation changes — text
// ---------------------------------------------------------------------------

#[test]
fn snapshot_deprecated_text() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/deprecated.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Text).unwrap();
    insta::assert_snapshot!("deprecated_text", output);
}

// ---------------------------------------------------------------------------
// No changes
// ---------------------------------------------------------------------------

#[test]
fn snapshot_no_changes_text() {
    let spec = load("formatter/empty.yaml");
    let report = compare(&spec, &spec);
    let output = format_report(&report, OutputFormat::Text).unwrap();
    insta::assert_snapshot!("no_changes_text", output);
}

// ---------------------------------------------------------------------------
// Filtered output
// ---------------------------------------------------------------------------

#[test]
fn snapshot_breaking_only_text() {
    let old = load("formatter/base.yaml");
    let new = load("formatter/breaking.yaml");
    let report = compare(&old, &new);
    let filtered = report.filtered(openapi_semantic_diff::model::Severity::Breaking);
    let output = format_report(&filtered, OutputFormat::Text).unwrap();
    insta::assert_snapshot!("breaking_only_text", output);
}

// ---------------------------------------------------------------------------
// Showcase — every severity × change-type across paths, params, schemas
// ---------------------------------------------------------------------------

#[test]
fn snapshot_showcase_markdown() {
    let old = load("showcase/base.yaml");
    let new = load("showcase/changed.yaml");
    let report = compare(&old, &new);
    let output = format_report(&report, OutputFormat::Markdown).unwrap();
    insta::assert_snapshot!("showcase_markdown", output);
}
