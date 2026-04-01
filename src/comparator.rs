// src/comparator.rs — Semantic comparison of two InternalSpecs
//
// Produces a `DiffReport` by walking both specs in parallel, matching
// entities by their semantic keys (not file position), and classifying
// every detected change with a severity tier.
//
// Severity rules:
//   Breaking    — Removed endpoints/fields/responses/params, narrowed types,
//                 new required constraints, removed enum values
//   Deprecated  — Something newly marked `deprecated: true`
//   Additive    — New endpoints/fields/responses/params, widened types,
//                 relaxed constraints, new enum values

use crate::extension::ExtensionRegistry;
use crate::model::*;
use indexmap::IndexMap;

/// Compare two specs and produce a diff report.
pub fn compare(old: &InternalSpec, new: &InternalSpec) -> DiffReport {
    compare_with_extensions(old, new, &ExtensionRegistry::with_defaults())
}

/// Compare two specs using a custom extension registry.
pub fn compare_with_extensions(
    old: &InternalSpec,
    new: &InternalSpec,
    extensions: &ExtensionRegistry,
) -> DiffReport {
    DiffReport {
        info_changes: compare_info(&old.info, &new.info),
        server_changes: compare_servers(&old.servers, &new.servers),
        path_changes: compare_paths(&old.paths, &new.paths, extensions),
        schema_changes: compare_schemas(
            "components.schemas",
            &old.components.schemas,
            &new.components.schemas,
            extensions,
        ),
        security_scheme_changes: compare_security_schemes(
            &old.components.security_schemes,
            &new.components.security_schemes,
        ),
        tag_changes: compare_tags(&old.tags, &new.tags),
        extension_changes: compare_extension_maps("", &old.extensions, &new.extensions, extensions),
    }
}

// ---------------------------------------------------------------------------
// Info
// ---------------------------------------------------------------------------

fn compare_info(old: &Info, new: &Info) -> Vec<Change> {
    let mut changes = Vec::new();

    if old.title != new.title {
        changes.push(Change {
            path: "info.title".to_string(),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: format!("title changed from '{}' to '{}'", old.title, new.title),
            old_value: Some(serde_json::json!(old.title)),
            new_value: Some(serde_json::json!(new.title)),
        });
    }

    if old.version != new.version {
        changes.push(Change {
            path: "info.version".to_string(),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: format!(
                "version changed from '{}' to '{}'",
                old.version, new.version
            ),
            old_value: Some(serde_json::json!(old.version)),
            new_value: Some(serde_json::json!(new.version)),
        });
    }

    if old.description != new.description {
        changes.push(Change {
            path: "info.description".to_string(),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: "description changed".to_string(),
            old_value: old.description.as_ref().map(|d| serde_json::json!(d)),
            new_value: new.description.as_ref().map(|d| serde_json::json!(d)),
        });
    }

    changes
}

// ---------------------------------------------------------------------------
// Servers
// ---------------------------------------------------------------------------

fn compare_servers(old: &[Server], new: &[Server]) -> Vec<Change> {
    let mut changes = Vec::new();

    // Key by URL
    let old_map: IndexMap<&str, &Server> = old.iter().map(|s| (s.url.as_str(), s)).collect();
    let new_map: IndexMap<&str, &Server> = new.iter().map(|s| (s.url.as_str(), s)).collect();

    for (url, _server) in &old_map {
        if !new_map.contains_key(url) {
            changes.push(Change {
                path: format!("servers.{url}"),
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("server '{url}' removed"),
                old_value: Some(serde_json::json!(url)),
                new_value: None,
            });
        }
    }

    for (url, _server) in &new_map {
        if !old_map.contains_key(url) {
            changes.push(Change {
                path: format!("servers.{url}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("server '{url}' added"),
                old_value: None,
                new_value: Some(serde_json::json!(url)),
            });
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// Paths & Operations
// ---------------------------------------------------------------------------

fn compare_paths(
    old: &IndexMap<String, PathItem>,
    new: &IndexMap<String, PathItem>,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    for (path, old_item) in old {
        if let Some(new_item) = new.get(path) {
            changes.extend(compare_path_item(path, old_item, new_item, extensions));
        } else {
            // Entire path removed
            for (method, _) in &old_item.operations {
                changes.push(Change {
                    path: format!("paths.{path}.{method}"),
                    change_type: ChangeType::Removed,
                    severity: Severity::Breaking,
                    message: format!("endpoint {method} {path} removed"),
                    old_value: None,
                    new_value: None,
                });
            }
        }
    }

    for (path, new_item) in new {
        if !old.contains_key(path) {
            for (method, _) in &new_item.operations {
                changes.push(Change {
                    path: format!("paths.{path}.{method}"),
                    change_type: ChangeType::Added,
                    severity: Severity::Additive,
                    message: format!("endpoint {method} {path} added"),
                    old_value: None,
                    new_value: None,
                });
            }
        }
    }

    changes
}

fn compare_path_item(
    path: &str,
    old: &PathItem,
    new: &PathItem,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Compare operations by method
    for (method, old_op) in &old.operations {
        let op_path = format!("paths.{path}.{method}");
        if let Some(new_op) = new.operations.get(method) {
            changes.extend(compare_operation(&op_path, old_op, new_op, extensions));
        } else {
            changes.push(Change {
                path: op_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("endpoint {method} {path} removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for (method, _) in &new.operations {
        if !old.operations.contains_key(method) {
            changes.push(Change {
                path: format!("paths.{path}.{method}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("endpoint {method} {path} added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    // Compare path-level extensions
    changes.extend(compare_extension_maps(
        &format!("paths.{path}"),
        &old.extensions,
        &new.extensions,
        extensions,
    ));

    changes
}

fn compare_operation(
    path: &str,
    old: &Operation,
    new: &Operation,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Deprecation
    if !old.deprecated && new.deprecated {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Deprecated,
            severity: Severity::Deprecated,
            message: "operation marked as deprecated".to_string(),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }

    // Summary change
    if old.summary != new.summary {
        changes.push(Change {
            path: format!("{path}.summary"),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: "summary changed".to_string(),
            old_value: old.summary.as_ref().map(|s| serde_json::json!(s)),
            new_value: new.summary.as_ref().map(|s| serde_json::json!(s)),
        });
    }

    // Description change
    if old.description != new.description {
        changes.push(Change {
            path: format!("{path}.description"),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: "description changed".to_string(),
            old_value: old.description.as_ref().map(|s| serde_json::json!(s)),
            new_value: new.description.as_ref().map(|s| serde_json::json!(s)),
        });
    }

    // Parameters (keyed by (name, location))
    changes.extend(compare_parameters(
        path,
        &old.parameters,
        &new.parameters,
        extensions,
    ));

    // Request body
    changes.extend(compare_request_body(
        path,
        &old.request_body,
        &new.request_body,
        extensions,
    ));

    // Responses (keyed by status code)
    changes.extend(compare_responses(
        path,
        &old.responses,
        &new.responses,
        extensions,
    ));

    // Operation extensions
    changes.extend(compare_extension_maps(
        path,
        &old.extensions,
        &new.extensions,
        extensions,
    ));

    changes
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

fn param_key(p: &Parameter) -> (String, ParameterLocation) {
    (p.name.clone(), p.location)
}

fn compare_parameters(
    path: &str,
    old: &[Parameter],
    new: &[Parameter],
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    let old_map: IndexMap<_, _> = old.iter().map(|p| (param_key(p), p)).collect();
    let new_map: IndexMap<_, _> = new.iter().map(|p| (param_key(p), p)).collect();

    for (key, old_param) in &old_map {
        let param_path = format!("{path}.parameters.{}.{}", key.0, key.1);
        if let Some(new_param) = new_map.get(key) {
            changes.extend(compare_parameter(
                &param_path,
                old_param,
                new_param,
                extensions,
            ));
        } else {
            changes.push(Change {
                path: param_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("parameter '{}' ({}) removed", key.0, key.1),
                old_value: None,
                new_value: None,
            });
        }
    }

    for (key, _) in &new_map {
        if !old_map.contains_key(key) {
            let param_path = format!("{path}.parameters.{}.{}", key.0, key.1);
            let severity = if new_map[key].required {
                Severity::Breaking
            } else {
                Severity::Additive
            };
            changes.push(Change {
                path: param_path,
                change_type: ChangeType::Added,
                severity,
                message: format!("parameter '{}' ({}) added", key.0, key.1),
                old_value: None,
                new_value: None,
            });
        }
    }

    changes
}

fn compare_parameter(
    path: &str,
    old: &Parameter,
    new: &Parameter,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Required: false → true is breaking
    if !old.required && new.required {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Modified,
            severity: Severity::Breaking,
            message: format!("parameter '{}' is now required", old.name),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }
    // Required: true → false is additive (relaxed)
    if old.required && !new.required {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: format!("parameter '{}' is no longer required", old.name),
            old_value: Some(serde_json::json!(true)),
            new_value: Some(serde_json::json!(false)),
        });
    }

    // Deprecation
    if !old.deprecated && new.deprecated {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Deprecated,
            severity: Severity::Deprecated,
            message: format!("parameter '{}' marked as deprecated", old.name),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }

    // Schema changes
    if let (Some(old_schema), Some(new_schema)) = (&old.schema, &new.schema) {
        changes.extend(compare_schema(
            &format!("{path}.schema"),
            old_schema,
            new_schema,
            extensions,
        ));
    }

    // Parameter extensions
    changes.extend(compare_extension_maps(
        path,
        &old.extensions,
        &new.extensions,
        extensions,
    ));

    changes
}

// ---------------------------------------------------------------------------
// Request Body
// ---------------------------------------------------------------------------

fn compare_request_body(
    path: &str,
    old: &Option<RequestBody>,
    new: &Option<RequestBody>,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let body_path = format!("{path}.requestBody");
    match (old, new) {
        (None, Some(_)) => vec![Change {
            path: body_path,
            change_type: ChangeType::Added,
            severity: Severity::Breaking, // adding a required body is breaking
            message: "request body added".to_string(),
            old_value: None,
            new_value: None,
        }],
        (Some(_), None) => vec![Change {
            path: body_path,
            change_type: ChangeType::Removed,
            severity: Severity::Breaking,
            message: "request body removed".to_string(),
            old_value: None,
            new_value: None,
        }],
        (Some(old_body), Some(new_body)) => {
            let mut changes = Vec::new();

            // Required flag
            if !old_body.required && new_body.required {
                changes.push(Change {
                    path: body_path.clone(),
                    change_type: ChangeType::Modified,
                    severity: Severity::Breaking,
                    message: "request body is now required".to_string(),
                    old_value: Some(serde_json::json!(false)),
                    new_value: Some(serde_json::json!(true)),
                });
            }

            // Compare content types
            for (media_type, old_mt) in &old_body.content {
                let mt_path = format!("{body_path}.content.{media_type}");
                if let Some(new_mt) = new_body.content.get(media_type) {
                    if let (Some(old_schema), Some(new_schema)) = (&old_mt.schema, &new_mt.schema) {
                        changes.extend(compare_schema(
                            &format!("{mt_path}.schema"),
                            old_schema,
                            new_schema,
                            extensions,
                        ));
                    }
                } else {
                    changes.push(Change {
                        path: mt_path,
                        change_type: ChangeType::Removed,
                        severity: Severity::Breaking,
                        message: format!("content type '{media_type}' removed from request body"),
                        old_value: None,
                        new_value: None,
                    });
                }
            }

            for media_type in new_body.content.keys() {
                if !old_body.content.contains_key(media_type) {
                    changes.push(Change {
                        path: format!("{body_path}.content.{media_type}"),
                        change_type: ChangeType::Added,
                        severity: Severity::Additive,
                        message: format!("content type '{media_type}' added to request body"),
                        old_value: None,
                        new_value: None,
                    });
                }
            }

            changes
        }
        (None, None) => vec![],
    }
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

fn compare_responses(
    path: &str,
    old: &IndexMap<String, Response>,
    new: &IndexMap<String, Response>,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    for (code, old_resp) in old {
        let resp_path = format!("{path}.responses.{code}");
        if let Some(new_resp) = new.get(code) {
            changes.extend(compare_response(&resp_path, old_resp, new_resp, extensions));
        } else {
            changes.push(Change {
                path: resp_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("response '{code}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for code in new.keys() {
        if !old.contains_key(code) {
            changes.push(Change {
                path: format!("{path}.responses.{code}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("response '{code}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    changes
}

fn compare_response(
    path: &str,
    old: &Response,
    new: &Response,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Compare content types
    for (media_type, old_mt) in &old.content {
        let mt_path = format!("{path}.content.{media_type}");
        if let Some(new_mt) = new.content.get(media_type) {
            if let (Some(old_schema), Some(new_schema)) = (&old_mt.schema, &new_mt.schema) {
                changes.extend(compare_schema(
                    &format!("{mt_path}.schema"),
                    old_schema,
                    new_schema,
                    extensions,
                ));
            }
        } else {
            changes.push(Change {
                path: mt_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("content type '{media_type}' removed from response"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for media_type in new.content.keys() {
        if !old.content.contains_key(media_type) {
            changes.push(Change {
                path: format!("{path}.content.{media_type}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("content type '{media_type}' added to response"),
                old_value: None,
                new_value: None,
            });
        }
    }

    // Compare headers
    for (name, old_header) in &old.headers {
        let h_path = format!("{path}.headers.{name}");
        if let Some(new_header) = new.headers.get(name) {
            changes.extend(compare_header(&h_path, old_header, new_header));
        } else {
            changes.push(Change {
                path: h_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("response header '{name}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for name in new.headers.keys() {
        if !old.headers.contains_key(name) {
            changes.push(Change {
                path: format!("{path}.headers.{name}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("response header '{name}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    // Compare response extensions
    changes.extend(compare_extension_maps(
        path,
        &old.extensions,
        &new.extensions,
        extensions,
    ));

    changes
}

fn compare_header(path: &str, old: &Header, new: &Header) -> Vec<Change> {
    let mut changes = Vec::new();

    if !old.deprecated && new.deprecated {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Deprecated,
            severity: Severity::Deprecated,
            message: "header marked as deprecated".to_string(),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }

    changes
}

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

fn compare_schemas(
    base_path: &str,
    old: &IndexMap<String, Schema>,
    new: &IndexMap<String, Schema>,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    for (name, old_schema) in old {
        let schema_path = format!("{base_path}.{name}");
        if let Some(new_schema) = new.get(name) {
            changes.extend(compare_schema(
                &schema_path,
                old_schema,
                new_schema,
                extensions,
            ));
        } else {
            changes.push(Change {
                path: schema_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("schema '{name}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for name in new.keys() {
        if !old.contains_key(name) {
            changes.push(Change {
                path: format!("{base_path}.{name}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("schema '{name}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    changes
}

fn compare_schema(
    path: &str,
    old: &Schema,
    new: &Schema,
    extensions: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Skip deep comparison for cyclic refs
    if old.cyclic_ref.is_some() || new.cyclic_ref.is_some() {
        return changes;
    }

    // Type change
    if old.schema_type != new.schema_type {
        changes.push(Change {
            path: format!("{path}.type"),
            change_type: ChangeType::Modified,
            severity: Severity::Breaking,
            message: format!(
                "type changed from '{}' to '{}'",
                old.schema_type
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "unspecified".to_string()),
                new.schema_type
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "unspecified".to_string()),
            ),
            old_value: old
                .schema_type
                .as_ref()
                .map(|t| serde_json::json!(t.to_string())),
            new_value: new
                .schema_type
                .as_ref()
                .map(|t| serde_json::json!(t.to_string())),
        });
    }

    // Format change
    if old.format != new.format {
        changes.push(Change {
            path: format!("{path}.format"),
            change_type: ChangeType::Modified,
            severity: Severity::Breaking,
            message: format!(
                "format changed from '{}' to '{}'",
                old.format.as_deref().unwrap_or("none"),
                new.format.as_deref().unwrap_or("none"),
            ),
            old_value: old.format.as_ref().map(|f| serde_json::json!(f)),
            new_value: new.format.as_ref().map(|f| serde_json::json!(f)),
        });
    }

    // Deprecation
    if !old.deprecated && new.deprecated {
        changes.push(Change {
            path: path.to_string(),
            change_type: ChangeType::Deprecated,
            severity: Severity::Deprecated,
            message: "schema marked as deprecated".to_string(),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }

    // Nullable: nullable → non-nullable is breaking
    if old.nullable && !new.nullable {
        changes.push(Change {
            path: format!("{path}.nullable"),
            change_type: ChangeType::Modified,
            severity: Severity::Breaking,
            message: "field is no longer nullable".to_string(),
            old_value: Some(serde_json::json!(true)),
            new_value: Some(serde_json::json!(false)),
        });
    }
    if !old.nullable && new.nullable {
        changes.push(Change {
            path: format!("{path}.nullable"),
            change_type: ChangeType::Modified,
            severity: Severity::Additive,
            message: "field is now nullable".to_string(),
            old_value: Some(serde_json::json!(false)),
            new_value: Some(serde_json::json!(true)),
        });
    }

    // Required fields
    compare_required_fields(path, old, new, &mut changes);

    // Properties (keyed by field name)
    compare_properties(path, old, new, extensions, &mut changes);

    // Enum values
    compare_enum_values(path, old, new, &mut changes);

    // Constraints
    compare_constraints(path, old, new, &mut changes);

    // Array items
    if let (Some(old_items), Some(new_items)) = (&old.items, &new.items) {
        changes.extend(compare_schema(
            &format!("{path}.items"),
            old_items,
            new_items,
            extensions,
        ));
    }

    // Schema extensions
    changes.extend(compare_extension_maps(
        path,
        &old.extensions,
        &new.extensions,
        extensions,
    ));

    changes
}

fn compare_required_fields(path: &str, old: &Schema, new: &Schema, changes: &mut Vec<Change>) {
    // New required fields that weren't required before = breaking
    for field in &new.required {
        if !old.required.contains(field) {
            // Check if the field even existed before
            let existed = old.properties.contains_key(field);
            if existed {
                changes.push(Change {
                    path: format!("{path}.required"),
                    change_type: ChangeType::Modified,
                    severity: Severity::Breaking,
                    message: format!("field '{field}' is now required"),
                    old_value: None,
                    new_value: Some(serde_json::json!(field)),
                });
            }
        }
    }

    // Previously required fields no longer required = additive (relaxed)
    for field in &old.required {
        if !new.required.contains(field) {
            // Only report if the field still exists
            if new.properties.contains_key(field) {
                changes.push(Change {
                    path: format!("{path}.required"),
                    change_type: ChangeType::Modified,
                    severity: Severity::Additive,
                    message: format!("field '{field}' is no longer required"),
                    old_value: Some(serde_json::json!(field)),
                    new_value: None,
                });
            }
        }
    }
}

fn compare_properties(
    path: &str,
    old: &Schema,
    new: &Schema,
    extensions: &ExtensionRegistry,
    changes: &mut Vec<Change>,
) {
    for (name, old_prop) in &old.properties {
        let prop_path = format!("{path}.properties.{name}");
        if let Some(new_prop) = new.properties.get(name) {
            changes.extend(compare_schema(&prop_path, old_prop, new_prop, extensions));
        } else {
            // Property removed — always breaking for consumers who rely on it
            let severity = Severity::Breaking;
            changes.push(Change {
                path: prop_path,
                change_type: ChangeType::Removed,
                severity,
                message: format!("property '{name}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for name in new.properties.keys() {
        if !old.properties.contains_key(name) {
            let prop_path = format!("{path}.properties.{name}");
            // New required property = breaking; new optional = additive
            let severity = if new.required.contains(name) {
                Severity::Breaking
            } else {
                Severity::Additive
            };
            changes.push(Change {
                path: prop_path,
                change_type: ChangeType::Added,
                severity,
                message: format!("property '{name}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }
}

fn compare_enum_values(path: &str, old: &Schema, new: &Schema, changes: &mut Vec<Change>) {
    if old.enum_values.is_empty() && new.enum_values.is_empty() {
        return;
    }

    let enum_path = format!("{path}.enum");

    for val in &old.enum_values {
        if !new.enum_values.contains(val) {
            changes.push(Change {
                path: enum_path.clone(),
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("enum value {} removed", val),
                old_value: Some(val.clone()),
                new_value: None,
            });
        }
    }

    for val in &new.enum_values {
        if !old.enum_values.contains(val) {
            changes.push(Change {
                path: enum_path.clone(),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("enum value {} added", val),
                old_value: None,
                new_value: Some(val.clone()),
            });
        }
    }
}

fn compare_constraints(path: &str, old: &Schema, new: &Schema, changes: &mut Vec<Change>) {
    // maxLength: reduced = breaking, increased = additive
    if let (Some(old_val), Some(new_val)) = (old.max_length, new.max_length) {
        if new_val < old_val {
            changes.push(Change {
                path: format!("{path}.maxLength"),
                change_type: ChangeType::Modified,
                severity: Severity::Breaking,
                message: format!("maxLength reduced from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        } else if new_val > old_val {
            changes.push(Change {
                path: format!("{path}.maxLength"),
                change_type: ChangeType::Modified,
                severity: Severity::Additive,
                message: format!("maxLength increased from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        }
    } else if old.max_length.is_some() && new.max_length.is_none() {
        // Constraint removed = additive (relaxed)
        changes.push(Change {
            path: format!("{path}.maxLength"),
            change_type: ChangeType::Removed,
            severity: Severity::Additive,
            message: "maxLength constraint removed".to_string(),
            old_value: old.max_length.map(|v| serde_json::json!(v)),
            new_value: None,
        });
    } else if old.max_length.is_none() && new.max_length.is_some() {
        // New constraint = breaking
        changes.push(Change {
            path: format!("{path}.maxLength"),
            change_type: ChangeType::Added,
            severity: Severity::Breaking,
            message: "maxLength constraint added".to_string(),
            old_value: None,
            new_value: new.max_length.map(|v| serde_json::json!(v)),
        });
    }

    // minLength: increased = breaking, reduced = additive
    if let (Some(old_val), Some(new_val)) = (old.min_length, new.min_length) {
        if new_val > old_val {
            changes.push(Change {
                path: format!("{path}.minLength"),
                change_type: ChangeType::Modified,
                severity: Severity::Breaking,
                message: format!("minLength increased from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        } else if new_val < old_val {
            changes.push(Change {
                path: format!("{path}.minLength"),
                change_type: ChangeType::Modified,
                severity: Severity::Additive,
                message: format!("minLength reduced from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        }
    }

    // maximum: reduced = breaking, increased = additive
    if let (Some(old_val), Some(new_val)) = (old.maximum, new.maximum) {
        if new_val < old_val {
            changes.push(Change {
                path: format!("{path}.maximum"),
                change_type: ChangeType::Modified,
                severity: Severity::Breaking,
                message: format!("maximum reduced from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        } else if new_val > old_val {
            changes.push(Change {
                path: format!("{path}.maximum"),
                change_type: ChangeType::Modified,
                severity: Severity::Additive,
                message: format!("maximum increased from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        }
    }

    // minimum: increased = breaking, reduced = additive
    if let (Some(old_val), Some(new_val)) = (old.minimum, new.minimum) {
        if new_val > old_val {
            changes.push(Change {
                path: format!("{path}.minimum"),
                change_type: ChangeType::Modified,
                severity: Severity::Breaking,
                message: format!("minimum increased from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        } else if new_val < old_val {
            changes.push(Change {
                path: format!("{path}.minimum"),
                change_type: ChangeType::Modified,
                severity: Severity::Additive,
                message: format!("minimum reduced from {old_val} to {new_val}"),
                old_value: Some(serde_json::json!(old_val)),
                new_value: Some(serde_json::json!(new_val)),
            });
        }
    }

    // pattern change = breaking
    if old.pattern != new.pattern && (old.pattern.is_some() || new.pattern.is_some()) {
        changes.push(Change {
            path: format!("{path}.pattern"),
            change_type: ChangeType::Modified,
            severity: Severity::Breaking,
            message: "pattern changed".to_string(),
            old_value: old.pattern.as_ref().map(|p| serde_json::json!(p)),
            new_value: new.pattern.as_ref().map(|p| serde_json::json!(p)),
        });
    }
}

// ---------------------------------------------------------------------------
// Security Schemes
// ---------------------------------------------------------------------------

fn compare_security_schemes(
    old: &IndexMap<String, SecurityScheme>,
    new: &IndexMap<String, SecurityScheme>,
) -> Vec<Change> {
    let mut changes = Vec::new();

    for (name, _old_scheme) in old {
        let scheme_path = format!("components.securitySchemes.{name}");
        if let Some(_new_scheme) = new.get(name) {
            // Could compare scheme details; for now just check presence
            // (detailed scheme comparison can be added later)
        } else {
            changes.push(Change {
                path: scheme_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("security scheme '{name}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for name in new.keys() {
        if !old.contains_key(name) {
            changes.push(Change {
                path: format!("components.securitySchemes.{name}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("security scheme '{name}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

fn compare_tags(old: &[Tag], new: &[Tag]) -> Vec<Change> {
    let mut changes = Vec::new();

    let old_map: IndexMap<&str, &Tag> = old.iter().map(|t| (t.name.as_str(), t)).collect();
    let new_map: IndexMap<&str, &Tag> = new.iter().map(|t| (t.name.as_str(), t)).collect();

    for (name, _) in &old_map {
        if !new_map.contains_key(name) {
            changes.push(Change {
                path: format!("tags.{name}"),
                change_type: ChangeType::Removed,
                severity: Severity::Additive, // tags are metadata, not contract
                message: format!("tag '{name}' removed"),
                old_value: None,
                new_value: None,
            });
        }
    }

    for (name, _) in &new_map {
        if !old_map.contains_key(name) {
            changes.push(Change {
                path: format!("tags.{name}"),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("tag '{name}' added"),
                old_value: None,
                new_value: None,
            });
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

fn compare_extension_maps(
    path: &str,
    old: &IndexMap<String, serde_json::Value>,
    new: &IndexMap<String, serde_json::Value>,
    registry: &ExtensionRegistry,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // All keys from both maps
    let mut all_keys: Vec<&String> = old.keys().chain(new.keys()).collect();
    all_keys.sort();
    all_keys.dedup();

    for key in all_keys {
        let old_val = old.get(key);
        let new_val = new.get(key);

        if old_val != new_val {
            changes.extend(registry.process(path, key, old_val, new_val));
        }
    }

    changes
}
