// src/formatter.rs — Tera template-based output formatting

use crate::model::{
    Change, ChangeEntry, DiffReport, GroupedReport, MetadataGroup, OsdError, PathGroup,
    PropertyGroup, VerbGroup,
};
use indexmap::IndexMap;
use tera::{Context, Tera};

/// Built-in templates, embedded at compile time.
const TEXT_TEMPLATE: &str = include_str!("../templates/text.tera");
const MARKDOWN_TEMPLATE: &str = include_str!("../templates/markdown.tera");
const JSON_TEMPLATE: &str = include_str!("../templates/json.tera");
const HTML_TEMPLATE: &str = include_str!("../templates/html.tera");

/// Supported built-in output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    #[value(alias = "md")]
    Markdown,
    Json,
    Html,
}

impl OutputFormat {
    pub fn template_name(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Html => "html",
        }
    }

    pub fn built_in_template(&self) -> &'static str {
        match self {
            Self::Text => TEXT_TEMPLATE,
            Self::Markdown => MARKDOWN_TEMPLATE,
            Self::Json => JSON_TEMPLATE,
            Self::Html => HTML_TEMPLATE,
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "html" => Ok(Self::Html),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Markdown => write!(f, "markdown"),
            Self::Json => write!(f, "json"),
            Self::Html => write!(f, "html"),
        }
    }
}

/// Format a diff report using a built-in template.
pub fn format_report(report: &DiffReport, format: OutputFormat) -> Result<String, OsdError> {
    let template = format.built_in_template();
    render(report, template)
}

/// Format a diff report using a custom template string.
pub fn format_report_custom(report: &DiffReport, template: &str) -> Result<String, OsdError> {
    render(report, template)
}

/// Export a built-in template's source.
pub fn export_template(format: OutputFormat) -> &'static str {
    format.built_in_template()
}

// ---------------------------------------------------------------------------
// Grouping logic
// ---------------------------------------------------------------------------

/// HTTP verb ordering for display purposes (REST-conventional order).
fn verb_order(verb: &str) -> usize {
    match verb {
        "GET" => 0,
        "POST" => 1,
        "PUT" => 2,
        "PATCH" => 3,
        "DELETE" => 4,
        "OPTIONS" => 5,
        "HEAD" => 6,
        "TRACE" => 7,
        _ => 8,
    }
}

/// ChangeType ordering: Added → Removed → Modified → Deprecated.
fn change_type_order(ct: &str) -> usize {
    match ct {
        "added" => 0,
        "removed" => 1,
        "modified" => 2,
        "deprecated" => 3,
        _ => 4,
    }
}

/// Parse an endpoint change path into `(route, verb, property)`.
///
/// Input path format: `paths.<route>.<VERB>[.<property>...]`
///
/// The route starts with `/` and never contains `.`, so the split is
/// unambiguous. Returns `None` if the path is not an endpoint path.
fn parse_endpoint_path(path: &str) -> Option<(String, String, String)> {
    // Must start with "paths." followed by a "/" character
    let rest = path.strip_prefix("paths.")?;
    if !rest.starts_with('/') {
        return None;
    }
    // The route is everything up to (but not including) the first ".<VERB>"
    // segment. Routes can contain "/" and "{}" but never ".".
    let dot_pos = rest.find('.')?;
    let route = rest[..dot_pos].to_string();
    let after_route = &rest[dot_pos + 1..];

    // Extract the HTTP verb — it's the next dot-delimited segment.
    let (verb, property) = match after_route.find('.') {
        Some(p) => (
            after_route[..p].to_string(),
            after_route[p + 1..].to_string(),
        ),
        None => (after_route.to_string(), String::new()),
    };

    Some((route, verb, property))
}

/// Parse a schema change path into `(schema_name, sub_path)`.
///
/// Input: `components.schemas.<SchemaName>[.<sub>...]`
fn parse_schema_path(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("components.schemas.")?;
    match rest.find('.') {
        Some(p) => Some((rest[..p].to_string(), rest[p + 1..].to_string())),
        None => Some((rest.to_string(), String::new())),
    }
}

/// For schema changes, attempt to find which API paths use that schema.
///
/// A schema change at `components.schemas.<Name>.<sub>` is considered to
/// "affect" an endpoint property if the endpoint property's sub-path ends with
/// exactly `schema.<sub>` or `schema.items.<sub>`.
///
/// Examples of correct matches:
///   schema_sub = "properties.id.type"
///   endpoint   = "responses.200.content.application/json.schema.items.properties.id.type"
///   → ends_with("schema.items.properties.id.type") ✓
///
///   schema_sub = "properties.name"
///   endpoint   = "responses.200.content.application/json.schema.properties.name"
///   → ends_with("schema.properties.name") ✓
///
/// Non-match (property is a child of the schema property, not the property itself):
///   schema_sub = "properties.name"
///   endpoint   = "requestBody.content.application/json.schema.properties.name.maxLength"
///   → does NOT end with "schema.properties.name" (ends with "maxLength") ✗
fn schema_matches_property(schema_sub: &str, endpoint_property: &str) -> bool {
    if schema_sub.is_empty() {
        return false;
    }
    // Exact suffix match (no wrapping .contains — avoids false positives).
    endpoint_property.ends_with(&format!("schema.{schema_sub}"))
        || endpoint_property.ends_with(&format!("schema.items.{schema_sub}"))
        || endpoint_property == schema_sub
}

/// Build a `GroupedReport` from a `DiffReport`.
///
/// Grouping rules:
/// - All `paths.*` changes → `GroupedReport::paths` (Path > Verb > Property)
/// - `components.schemas.*` changes that match a path change are inlined into
///   the relevant `PathGroup` with a note; the remaining schema changes go to
///   `GroupedReport::metadata`.
/// - All other changes → `GroupedReport::metadata` grouped by category/item.
///
/// Within every level, items are sorted:
/// - Routes: alphabetically
/// - Verbs: REST-conventional order (GET, POST, PUT, PATCH, DELETE, …)
/// - Properties: alphabetically
/// - Changes within a property group: Added → Removed → Modified → Deprecated
pub fn build_grouped_report(report: &DiffReport) -> GroupedReport {
    let all: Vec<&Change> = report.all_changes();
    let total = all.len();
    let max_sev = report.max_severity().map(|s| s.to_string());

    // -----------------------------------------------------------------------
    // Step 1: Bucket path changes into (route, verb, property) triples.
    // -----------------------------------------------------------------------
    // Key: (route, verb, property) → Vec<ChangeEntry>
    let mut endpoint_map: IndexMap<(String, String, String), Vec<ChangeEntry>> = IndexMap::new();

    for change in &report.path_changes {
        if let Some((route, verb, property)) = parse_endpoint_path(&change.path) {
            endpoint_map
                .entry((route, verb, property))
                .or_default()
                .push(ChangeEntry::from(change));
        }
    }

    // -----------------------------------------------------------------------
    // Step 2: For each schema change, check whether it affects any path and
    // if so inline it into the relevant endpoint property groups.
    // -----------------------------------------------------------------------
    let mut inlined_schema_paths: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for schema_change in &report.schema_changes {
        if let Some((schema_name, schema_sub)) = parse_schema_path(&schema_change.path) {
            if schema_sub.is_empty() {
                // Whole-schema add/remove — can't meaningfully inline.
                continue;
            }
            // Find all endpoint properties whose sub-path structurally matches.
            let keys: Vec<(String, String, String)> = endpoint_map.keys().cloned().collect();
            for key in &keys {
                let (_, _, ref prop) = *key;
                if schema_matches_property(&schema_sub, prop) {
                    // Build an annotated entry that references the schema.
                    let mut entry = ChangeEntry::from(schema_change);
                    entry.message = format!("[schema: {}] {}", schema_name, schema_change.message);
                    endpoint_map.entry(key.clone()).or_default().push(entry);
                    inlined_schema_paths.insert(schema_change.path.clone());
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Step 3: Sort changes within each property group, then assemble
    // PathGroup / VerbGroup / PropertyGroup hierarchy.
    // -----------------------------------------------------------------------
    // Sort the keys for deterministic output.
    let mut sorted_keys: Vec<(String, String, String)> = endpoint_map.keys().cloned().collect();
    sorted_keys.sort_by(|a, b| {
        // Primary: route alphabetically
        let r = a.0.cmp(&b.0);
        if r != std::cmp::Ordering::Equal {
            return r;
        }
        // Secondary: verb by REST-conventional order
        let vr = verb_order(&a.1).cmp(&verb_order(&b.1));
        if vr != std::cmp::Ordering::Equal {
            return vr;
        }
        // Tertiary: property alphabetically
        a.2.cmp(&b.2)
    });

    // Build nested structure using IndexMap to accumulate per-route/verb.
    // route → verb → property → [ChangeEntry]
    let mut routes: IndexMap<String, IndexMap<String, IndexMap<String, Vec<ChangeEntry>>>> =
        IndexMap::new();

    for (route, verb, property) in &sorted_keys {
        let entries = endpoint_map
            .shift_remove(&(route.clone(), verb.clone(), property.clone()))
            .unwrap_or_default();
        let mut sorted_entries = entries;
        sorted_entries.sort_by_key(|e| change_type_order(&e.change_type));

        routes
            .entry(route.clone())
            .or_default()
            .entry(verb.clone())
            .or_default()
            .insert(property.clone(), sorted_entries);
    }

    let path_groups: Vec<PathGroup> = routes
        .into_iter()
        .map(|(route, verbs)| {
            let mut verb_list: Vec<(&String, &IndexMap<String, Vec<ChangeEntry>>)> =
                verbs.iter().collect();
            // verbs are already in insertion order (which is sorted_keys order,
            // so REST-conventional). Sort again explicitly for safety.
            verb_list.sort_by_key(|(v, _)| verb_order(v));

            let verb_groups: Vec<VerbGroup> = verbs
                .into_iter()
                .map(|(verb, props)| {
                    let mut prop_list: Vec<(String, Vec<ChangeEntry>)> =
                        props.into_iter().collect();
                    prop_list.sort_by(|(a, _), (b, _)| a.cmp(b));

                    let property_groups: Vec<PropertyGroup> = prop_list
                        .into_iter()
                        .map(|(property, changes)| PropertyGroup { property, changes })
                        .collect();

                    VerbGroup {
                        verb,
                        property_groups,
                    }
                })
                .collect();

            // Re-sort verb_groups by REST order since IndexMap preserves
            // insertion order but we want to guarantee.
            let mut vg = verb_groups;
            vg.sort_by_key(|v| verb_order(&v.verb));

            PathGroup {
                route,
                verb_groups: vg,
            }
        })
        .collect();

    // -----------------------------------------------------------------------
    // Step 4: Build metadata groups for all non-path, non-inlined changes.
    // -----------------------------------------------------------------------
    let mut metadata: Vec<MetadataGroup> = Vec::new();

    // Helper: add a group for a slice of changes under a given label,
    // grouping by property (the part after the category prefix).
    let make_metadata_group =
        |label: &str, changes: &[Change], prefix: &str| -> Option<MetadataGroup> {
            if changes.is_empty() {
                return None;
            }
            // Group by property sub-path (alphabetically).
            let mut prop_map: IndexMap<String, Vec<ChangeEntry>> = IndexMap::new();
            for c in changes {
                let property = c.path.strip_prefix(prefix).unwrap_or(&c.path).to_string();
                prop_map
                    .entry(property)
                    .or_default()
                    .push(ChangeEntry::from(c));
            }
            let mut prop_list: Vec<(String, Vec<ChangeEntry>)> = prop_map.into_iter().collect();
            prop_list.sort_by(|(a, _), (b, _)| a.cmp(b));
            // Sort changes within each property group.
            let property_groups: Vec<PropertyGroup> = prop_list
                .into_iter()
                .map(|(property, mut changes)| {
                    changes.sort_by_key(|e| change_type_order(&e.change_type));
                    PropertyGroup { property, changes }
                })
                .collect();

            Some(MetadataGroup {
                label: label.to_string(),
                property_groups,
            })
        };

    // Info
    if let Some(g) = make_metadata_group("Info", &report.info_changes, "info.") {
        metadata.push(g);
    }

    // Servers
    if let Some(g) = make_metadata_group("Servers", &report.server_changes, "servers.") {
        metadata.push(g);
    }

    // Schemas — grouped per schema name, with inlined ones already handled.
    // Build a group per schema name for changes that weren't inlined.
    {
        let mut schema_by_name: IndexMap<String, Vec<&Change>> = IndexMap::new();
        for c in &report.schema_changes {
            if inlined_schema_paths.contains(&c.path) {
                continue; // already inlined into path groups
            }
            let name = parse_schema_path(&c.path)
                .map(|(n, _)| n)
                .unwrap_or_else(|| c.path.clone());
            schema_by_name.entry(name).or_default().push(c);
        }
        let mut names: Vec<String> = schema_by_name.keys().cloned().collect();
        names.sort();
        for name in names {
            let changes: Vec<Change> = schema_by_name[&name].iter().map(|c| (*c).clone()).collect();
            let label = format!("Schemas \u{203a} {name}");
            if let Some(g) =
                make_metadata_group(&label, &changes, &format!("components.schemas.{name}."))
            {
                metadata.push(g);
            }
        }
    }

    // Security schemes
    if let Some(g) = make_metadata_group(
        "Security Schemes",
        &report.security_scheme_changes,
        "components.securitySchemes.",
    ) {
        metadata.push(g);
    }

    // Tags
    if let Some(g) = make_metadata_group("Tags", &report.tag_changes, "tags.") {
        metadata.push(g);
    }

    // Extensions
    if let Some(g) = make_metadata_group("Extensions", &report.extension_changes, ".") {
        metadata.push(g);
    }

    GroupedReport {
        total_changes: total,
        max_severity: max_sev,
        paths: path_groups,
        metadata,
    }
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

fn render(report: &DiffReport, template_str: &str) -> Result<String, OsdError> {
    let mut tera = Tera::default();
    tera.add_raw_template("report", template_str)
        .map_err(|e| OsdError::Template(format!("template parse error: {e}")))?;

    let mut context = Context::new();

    // Flatten all changes into a single list for template access (kept for
    // backward-compat and for json.tera which stays flat).
    let changes: Vec<&Change> = report.all_changes();
    let serializable_changes: Vec<serde_json::Value> = changes
        .iter()
        .map(|c| serde_json::to_value(c).unwrap())
        .collect();

    context.insert("changes", &serializable_changes);
    context.insert(
        "max_severity",
        &report.max_severity().map(|s| s.to_string()),
    );

    // Also insert per-category changes for templates that want them
    context.insert(
        "info_changes",
        &serde_json::to_value(&report.info_changes).unwrap(),
    );
    context.insert(
        "server_changes",
        &serde_json::to_value(&report.server_changes).unwrap(),
    );
    context.insert(
        "path_changes",
        &serde_json::to_value(&report.path_changes).unwrap(),
    );
    context.insert(
        "schema_changes",
        &serde_json::to_value(&report.schema_changes).unwrap(),
    );
    context.insert(
        "security_scheme_changes",
        &serde_json::to_value(&report.security_scheme_changes).unwrap(),
    );
    context.insert(
        "tag_changes",
        &serde_json::to_value(&report.tag_changes).unwrap(),
    );
    context.insert(
        "extension_changes",
        &serde_json::to_value(&report.extension_changes).unwrap(),
    );

    // Grouped / human-readable structure for text, markdown, html templates.
    let grouped = build_grouped_report(report);
    context.insert("grouped", &serde_json::to_value(&grouped).unwrap());

    tera.render("report", &context)
        .map_err(|e| OsdError::Template(format!("template render error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn empty_report() -> DiffReport {
        DiffReport {
            info_changes: vec![],
            server_changes: vec![],
            path_changes: vec![],
            schema_changes: vec![],
            security_scheme_changes: vec![],
            tag_changes: vec![],
            extension_changes: vec![],
        }
    }

    fn sample_report() -> DiffReport {
        DiffReport {
            info_changes: vec![],
            server_changes: vec![],
            path_changes: vec![Change {
                path: "paths./pets.GET".to_string(),
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: "endpoint GET /pets removed".to_string(),
                old_value: None,
                new_value: None,
            }],
            schema_changes: vec![Change {
                path: "components.schemas.Pet.properties.name".to_string(),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: "property 'name' added".to_string(),
                old_value: None,
                new_value: None,
            }],
            security_scheme_changes: vec![],
            tag_changes: vec![],
            extension_changes: vec![],
        }
    }

    #[test]
    fn text_format_empty() {
        let output = format_report(&empty_report(), OutputFormat::Text).unwrap();
        assert!(output.contains("No changes"));
    }

    #[test]
    fn text_format_with_changes() {
        let output = format_report(&sample_report(), OutputFormat::Text).unwrap();
        assert!(output.contains("breaking"));
        assert!(output.contains("endpoint GET /pets removed"));
    }

    #[test]
    fn markdown_format() {
        let output = format_report(&sample_report(), OutputFormat::Markdown).unwrap();
        assert!(output.contains("## API Changes"));
        assert!(output.contains("## Paths"));
        assert!(output.contains("/pets"));
    }

    #[test]
    fn html_format() {
        let output = format_report(&sample_report(), OutputFormat::Html).unwrap();
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("badge-breaking"));
        assert!(output.contains("<h2>Paths</h2>"));
    }

    #[test]
    fn json_format() {
        let output = format_report(&sample_report(), OutputFormat::Json).unwrap();
        assert!(output.contains("\"total_changes\": 2"));
        assert!(output.contains("\"severity\": \"breaking\""));
    }

    #[test]
    fn export_template_returns_content() {
        let tmpl = export_template(OutputFormat::Text);
        assert!(tmpl.contains("change"));
    }

    #[test]
    fn custom_template() {
        let template = "Total: {{ changes | length }}";
        let output = format_report_custom(&sample_report(), template).unwrap();
        assert_eq!(output.trim(), "Total: 2");
    }

    #[test]
    fn parse_endpoint_path_basic() {
        let (route, verb, prop) =
            parse_endpoint_path("paths./pets.GET.parameters.limit.query").unwrap();
        assert_eq!(route, "/pets");
        assert_eq!(verb, "GET");
        assert_eq!(prop, "parameters.limit.query");
    }

    #[test]
    fn parse_endpoint_path_no_property() {
        let (route, verb, prop) = parse_endpoint_path("paths./pets/{petId}.DELETE").unwrap();
        assert_eq!(route, "/pets/{petId}");
        assert_eq!(verb, "DELETE");
        assert_eq!(prop, "");
    }

    #[test]
    fn parse_schema_path_basic() {
        let (name, sub) = parse_schema_path("components.schemas.Pet.properties.id.type").unwrap();
        assert_eq!(name, "Pet");
        assert_eq!(sub, "properties.id.type");
    }

    #[test]
    fn grouped_report_paths_sorted() {
        let report = DiffReport {
            path_changes: vec![
                Change {
                    path: "paths./z.GET".to_string(),
                    change_type: ChangeType::Added,
                    severity: Severity::Additive,
                    message: "z added".to_string(),
                    old_value: None,
                    new_value: None,
                },
                Change {
                    path: "paths./a.POST".to_string(),
                    change_type: ChangeType::Added,
                    severity: Severity::Additive,
                    message: "a added".to_string(),
                    old_value: None,
                    new_value: None,
                },
            ],
            info_changes: vec![],
            server_changes: vec![],
            schema_changes: vec![],
            security_scheme_changes: vec![],
            tag_changes: vec![],
            extension_changes: vec![],
        };
        let grouped = build_grouped_report(&report);
        assert_eq!(grouped.paths[0].route, "/a");
        assert_eq!(grouped.paths[1].route, "/z");
    }
}
