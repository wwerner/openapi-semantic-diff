// src/extension/extensible_enum.rs — x-extensible-enum processor
//
// The `x-extensible-enum` extension is a common convention for declaring
// enum values that are expected to grow over time.  Consumers should handle
// unknown values gracefully.
//
// Severity rules:
//   - Values added   → Additive
//   - Values removed → Breaking (consumers may depend on removed values)
//   - Order changes  → Ignored (order is not semantically significant)

use super::ExtensionProcessor;
use crate::model::{Change, ChangeType, Severity};

pub struct ExtensibleEnumProcessor;

impl ExtensionProcessor for ExtensibleEnumProcessor {
    fn key(&self) -> &str {
        "x-extensible-enum"
    }

    fn process(
        &self,
        path: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Vec<Change> {
        let ext_path = format!("{path}.x-extensible-enum");

        match (old_value, new_value) {
            (None, Some(new)) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: "x-extensible-enum added".to_string(),
                old_value: None,
                new_value: Some(new.clone()),
            }],
            (Some(old), None) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: "x-extensible-enum removed".to_string(),
                old_value: Some(old.clone()),
                new_value: None,
            }],
            (Some(old), Some(new)) => diff_enum_values(&ext_path, old, new),
            (None, None) => vec![],
        }
    }
}

fn diff_enum_values(path: &str, old: &serde_json::Value, new: &serde_json::Value) -> Vec<Change> {
    let old_values = extract_string_values(old);
    let new_values = extract_string_values(new);

    let mut changes = Vec::new();

    // Detect removed values (breaking)
    for val in &old_values {
        if !new_values.contains(val) {
            changes.push(Change {
                path: path.to_string(),
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: format!("enum value '{val}' removed from x-extensible-enum"),
                old_value: Some(serde_json::Value::String(val.clone())),
                new_value: None,
            });
        }
    }

    // Detect added values (additive)
    for val in &new_values {
        if !old_values.contains(val) {
            changes.push(Change {
                path: path.to_string(),
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("enum value '{val}' added to x-extensible-enum"),
                old_value: None,
                new_value: Some(serde_json::Value::String(val.clone())),
            });
        }
    }

    changes
}

fn extract_string_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_values_are_additive() {
        let old = serde_json::json!(["a", "b"]);
        let new = serde_json::json!(["a", "b", "c"]);
        let processor = ExtensibleEnumProcessor;
        let changes = processor.process("test", Some(&old), Some(&new));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].severity, Severity::Additive);
        assert_eq!(changes[0].change_type, ChangeType::Added);
    }

    #[test]
    fn removed_values_are_breaking() {
        let old = serde_json::json!(["a", "b", "c"]);
        let new = serde_json::json!(["a", "b"]);
        let processor = ExtensibleEnumProcessor;
        let changes = processor.process("test", Some(&old), Some(&new));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].severity, Severity::Breaking);
        assert_eq!(changes[0].change_type, ChangeType::Removed);
    }

    #[test]
    fn no_change() {
        let old = serde_json::json!(["a", "b"]);
        let new = serde_json::json!(["a", "b"]);
        let processor = ExtensibleEnumProcessor;
        let changes = processor.process("test", Some(&old), Some(&new));
        assert!(changes.is_empty());
    }

    #[test]
    fn entire_enum_removed_is_breaking() {
        let old = serde_json::json!(["a", "b"]);
        let processor = ExtensibleEnumProcessor;
        let changes = processor.process("test", Some(&old), None);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].severity, Severity::Breaking);
    }

    #[test]
    fn entire_enum_added_is_additive() {
        let new = serde_json::json!(["a", "b"]);
        let processor = ExtensibleEnumProcessor;
        let changes = processor.process("test", None, Some(&new));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].severity, Severity::Additive);
    }
}
