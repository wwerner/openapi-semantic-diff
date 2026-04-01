// src/main.rs — CLI entry point for osd

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use openapi_semantic_diff::comparator;
use openapi_semantic_diff::formatter::{self, OutputFormat};
use openapi_semantic_diff::model::{DiffReport, Severity};
use openapi_semantic_diff::parser;
use std::path::{Path, PathBuf};

/// Semantic diff for OpenAPI specifications.
///
/// Compares two OpenAPI specs and reports structured, classified changesets.
/// Supports OpenAPI 3.0.x and 3.1.x in YAML or JSON format.
#[derive(Parser)]
#[command(
    name = "osd",
    version,
    about,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare two OpenAPI specs and report changes
    Diff {
        /// Path to the old (base) OpenAPI spec
        old: PathBuf,

        /// Path to the new (changed) OpenAPI spec
        new: PathBuf,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,

        /// Custom Tera template file (overrides --format)
        #[arg(short, long)]
        template: Option<PathBuf>,

        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Minimum severity to include in output
        #[arg(long, default_value = "additive")]
        min_severity: Severity,
    },

    /// Exit 1 if breaking changes are found, 0 otherwise (CI gate)
    Check {
        /// Path to the old (base) OpenAPI spec
        old: PathBuf,

        /// Path to the new (changed) OpenAPI spec
        new: PathBuf,
    },

    /// Export a built-in template to a file for customization
    Templates {
        /// Template format to export
        format: OutputFormat,

        /// Output file path
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Diff {
            old,
            new,
            format,
            template,
            output,
            min_severity,
        } => run_diff(
            &old,
            &new,
            format,
            template.as_ref(),
            output.as_ref(),
            min_severity,
        ),
        Commands::Check { old, new } => run_check(&old, &new),
        Commands::Templates { format, output } => export_template(format, &output),
    };

    match result {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

/// Parse and compare two specs, returning the diff report.
fn parse_and_compare(old: &Path, new: &Path) -> Result<DiffReport> {
    let old_spec = parser::parse_file(old)
        .with_context(|| format!("failed to parse old spec: {}", old.display()))?;
    let new_spec = parser::parse_file(new)
        .with_context(|| format!("failed to parse new spec: {}", new.display()))?;
    Ok(comparator::compare(&old_spec, &new_spec))
}

/// `osd diff` — compare two specs and print a report. Always exits 0 on success.
fn run_diff(
    old: &Path,
    new: &Path,
    format: OutputFormat,
    template: Option<&PathBuf>,
    output: Option<&PathBuf>,
    min_severity: Severity,
) -> Result<i32> {
    let report = parse_and_compare(old, new)?;

    // Apply severity filter
    let report = report.filtered(min_severity);

    // Format output
    let rendered = if let Some(template_path) = template {
        let tmpl = std::fs::read_to_string(template_path)
            .with_context(|| format!("failed to read template: {}", template_path.display()))?;
        formatter::format_report_custom(&report, &tmpl)?
    } else {
        formatter::format_report(&report, format)?
    };

    // Write output
    if let Some(output_path) = output {
        std::fs::write(output_path, &rendered)
            .with_context(|| format!("failed to write output: {}", output_path.display()))?;
    } else {
        print!("{rendered}");
    }

    Ok(0)
}

/// `osd check` — exit 1 if breaking changes exist, 0 otherwise.
fn run_check(old: &Path, new: &Path) -> Result<i32> {
    let report = parse_and_compare(old, new)?;
    let breaking = report.filtered(Severity::Breaking);

    if breaking.is_empty() {
        eprintln!("no breaking changes");
        Ok(0)
    } else {
        let count = breaking.all_changes().len();
        eprintln!("{count} breaking change(s) found");
        Ok(1)
    }
}

/// `osd templates` — write a built-in template to disk.
fn export_template(format: OutputFormat, output: &Path) -> Result<i32> {
    let template = formatter::export_template(format);
    std::fs::write(output, template)
        .with_context(|| format!("failed to write template: {}", output.display()))?;
    eprintln!("exported {format} template to {}", output.display());
    Ok(0)
}
