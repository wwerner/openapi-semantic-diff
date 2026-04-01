// src/ref_resolver.rs — $ref inlining and cycle detection
//
// Works on raw `serde_json::Value` before typed parsing.  Walks the tree,
// finds `{"$ref": "..."}` nodes, loads the referenced content (from the same
// file or a sibling file on disk), and replaces the `$ref` node with the
// resolved content.  Tracks visited refs to detect cycles and inserts a
// `{"x-osd-cyclic-ref": "<name>"}` marker instead of recursing infinitely.

use crate::model::OsdError;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolve all `$ref`s in a JSON `Value` tree, inlining their content.
/// `base_dir` is the directory containing the root spec file (for resolving
/// relative file paths).  If `None`, cross-file refs produce an error.
pub fn resolve_refs(value: &mut Value, base_dir: Option<&Path>) -> Result<(), OsdError> {
    let mut visited = HashSet::new();
    resolve_recursive(value, base_dir, &mut visited)
}

fn resolve_recursive(
    value: &mut Value,
    base_dir: Option<&Path>,
    visited: &mut HashSet<String>,
) -> Result<(), OsdError> {
    match value {
        Value::Object(map) => {
            if let Some(ref_val) = map.get("$ref").cloned() {
                if let Some(ref_str) = ref_val.as_str() {
                    let ref_str = ref_str.to_string();

                    // Cycle detection
                    if visited.contains(&ref_str) {
                        let name = ref_str.rsplit('/').next().unwrap_or(&ref_str);
                        *value = serde_json::json!({
                            "x-osd-cyclic-ref": name
                        });
                        return Ok(());
                    }

                    visited.insert(ref_str.clone());
                    let mut resolved = resolve_ref(&ref_str, base_dir)?;
                    resolve_recursive(&mut resolved, base_dir, visited)?;
                    visited.remove(&ref_str);

                    *value = resolved;
                }
            } else {
                // Not a $ref object — recurse into all values
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(v) = map.get_mut(&key) {
                        resolve_recursive(v, base_dir, visited)?;
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_recursive(item, base_dir, visited)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Resolve a single `$ref` string to a `Value`.
fn resolve_ref(ref_str: &str, base_dir: Option<&Path>) -> Result<Value, OsdError> {
    if ref_str.starts_with("http://") || ref_str.starts_with("https://") {
        return Err(OsdError::RemoteRef(ref_str.to_string()));
    }

    // Split into file path and JSON pointer: "file.yaml#/components/schemas/Pet"
    let (file_part, pointer_part) = if let Some(idx) = ref_str.find('#') {
        (&ref_str[..idx], &ref_str[idx + 1..])
    } else {
        (ref_str, "")
    };

    // Load the document
    let doc = if file_part.is_empty() {
        // Same-file ref — this shouldn't happen because we inline in-place,
        // but if it does we return an error rather than panicking.
        return Err(OsdError::RefResolution(format!(
            "same-file ref '{ref_str}' encountered during resolution — this is a bug"
        )));
    } else {
        // Cross-file ref
        let base = base_dir.ok_or_else(|| {
            OsdError::RefResolution(format!(
                "cannot resolve cross-file ref '{ref_str}' without a base directory"
            ))
        })?;
        let file_path = base.join(file_part);
        load_file(&file_path)?
    };

    // Follow JSON pointer
    if pointer_part.is_empty() || pointer_part == "/" {
        Ok(doc)
    } else {
        resolve_pointer(&doc, pointer_part).ok_or_else(|| {
            OsdError::RefResolution(format!(
                "JSON pointer '{pointer_part}' not found in '{file_part}'"
            ))
        })
    }
}

fn resolve_pointer(doc: &Value, pointer: &str) -> Option<Value> {
    let pointer = pointer.strip_prefix('/').unwrap_or(pointer);
    let mut current = doc;
    for segment in pointer.split('/') {
        // Unescape JSON pointer encoding
        let segment = segment.replace("~1", "/").replace("~0", "~");
        current = current.get(&segment)?;
    }
    Some(current.clone())
}

fn load_file(path: &PathBuf) -> Result<Value, OsdError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        OsdError::RefResolution(format!("failed to read '{}': {e}", path.display()))
    })?;

    // Try JSON first, then YAML
    if let Ok(val) = serde_json::from_str::<Value>(&content) {
        return Ok(val);
    }
    serde_yml::from_str::<Value>(&content)
        .map_err(|e| OsdError::Yaml(format!("failed to parse '{}': {e}", path.display())))
}

/// Pre-process a spec string: parse to Value, resolve all `$ref`s, return the
/// fully-inlined Value.  This is the main entry point used by parsers.
pub fn preprocess(content: &str, source_path: Option<&Path>) -> Result<Value, OsdError> {
    // Parse to generic Value
    let mut value: Value = if content.trim_start().starts_with('{') {
        serde_json::from_str(content)?
    } else {
        serde_yml::from_str::<Value>(content).map_err(|e| OsdError::Yaml(e.to_string()))?
    };

    // Determine base directory for cross-file refs
    let base_dir = source_path.and_then(|p| p.parent());

    // Resolve all same-file refs by walking JSON pointers against the root doc.
    // We do this in a separate pass because same-file refs point into the same
    // document and don't need file I/O.
    resolve_same_file_refs(&mut value)?;

    // Now resolve any remaining cross-file refs
    resolve_refs(&mut value, base_dir)?;

    Ok(value)
}

/// Resolve same-file `$ref`s (those starting with `#/`).
fn resolve_same_file_refs(root: &mut Value) -> Result<(), OsdError> {
    // We need the root as a read-only reference for lookups, but we also need
    // to mutate it.  Clone the root for lookups.  This is acceptable because
    // we only do it once per parse.
    let root_snapshot = root.clone();
    let mut visited = HashSet::new();
    resolve_same_file_recursive(root, &root_snapshot, &mut visited)
}

fn resolve_same_file_recursive(
    value: &mut Value,
    root: &Value,
    visited: &mut HashSet<String>,
) -> Result<(), OsdError> {
    match value {
        Value::Object(map) => {
            if let Some(ref_val) = map.get("$ref").cloned() {
                if let Some(ref_str) = ref_val.as_str() {
                    if ref_str.starts_with("#/") {
                        let ref_str = ref_str.to_string();

                        if visited.contains(&ref_str) {
                            let name = ref_str.rsplit('/').next().unwrap_or(&ref_str);
                            *value = serde_json::json!({
                                "x-osd-cyclic-ref": name
                            });
                            return Ok(());
                        }

                        visited.insert(ref_str.clone());
                        let pointer = &ref_str[1..]; // strip leading '#'
                        let mut resolved = resolve_pointer(root, pointer).ok_or_else(|| {
                            OsdError::RefResolution(format!(
                                "JSON pointer '{pointer}' not found in document"
                            ))
                        })?;
                        resolve_same_file_recursive(&mut resolved, root, visited)?;
                        visited.remove(&ref_str);

                        *value = resolved;
                    }
                    // Non-same-file refs are left for the cross-file pass
                }
            } else {
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(v) = map.get_mut(&key) {
                        resolve_same_file_recursive(v, root, visited)?;
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_same_file_recursive(item, root, visited)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_same_file_ref() {
        let input = r##"{
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" }
                        }
                    }
                }
            },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": { "$ref": "#/components/schemas/Pet" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }"##;

        let mut value: Value = serde_json::from_str(input).unwrap();
        resolve_same_file_refs(&mut value).unwrap();

        // The $ref should be replaced with the actual Pet schema
        let schema = &value["paths"]["/pets"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"];
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
    }

    #[test]
    fn detect_cyclic_ref() {
        let input = r##"{
            "components": {
                "schemas": {
                    "Node": {
                        "type": "object",
                        "properties": {
                            "child": { "$ref": "#/components/schemas/Node" }
                        }
                    }
                }
            }
        }"##;

        let mut value: Value = serde_json::from_str(input).unwrap();
        resolve_same_file_refs(&mut value).unwrap();

        // First level: child is resolved to the full Node schema
        let child = &value["components"]["schemas"]["Node"]["properties"]["child"];
        assert_eq!(
            child["type"], "object",
            "child should be resolved to Node schema"
        );

        // Second level: the nested child should be the cyclic sentinel
        let nested_child = &child["properties"]["child"];
        assert_eq!(nested_child["x-osd-cyclic-ref"], "Node");
    }

    #[test]
    fn remote_ref_rejected() {
        let result = resolve_ref("https://example.com/spec.yaml#/Foo", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, OsdError::RemoteRef(_)));
    }
}
