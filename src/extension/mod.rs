// src/extension/mod.rs — ExtensionProcessor trait and registry

pub mod extensible_enum;

use crate::model::{Change, ChangeType, Severity};

/// Trait for processing `x-*` extension changes between old and new specs.
/// Each implementation handles a specific extension key and determines the
/// severity of changes to that extension's value.
pub trait ExtensionProcessor: Send + Sync {
    /// The extension key this processor handles (e.g. "x-extensible-enum").
    fn key(&self) -> &str;

    /// Produce changes for this extension given the old and new values.
    /// `path` is the dotted path to the containing object (e.g. "components.schemas.Pet").
    fn process(
        &self,
        path: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Vec<Change>;
}

/// Default processor for any `x-*` extension without a dedicated handler.
/// All changes are classified as `Severity::Additive`.
pub struct DefaultExtensionProcessor;

impl DefaultExtensionProcessor {
    pub fn process_extension(
        path: &str,
        key: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Vec<Change> {
        let ext_path = format!("{path}.{key}");
        match (old_value, new_value) {
            (None, Some(new)) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: format!("extension {key} added"),
                old_value: None,
                new_value: Some(new.clone()),
            }],
            (Some(old), None) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Removed,
                severity: Severity::Additive,
                message: format!("extension {key} removed"),
                old_value: Some(old.clone()),
                new_value: None,
            }],
            (Some(old), Some(new)) if old != new => vec![Change {
                path: ext_path,
                change_type: ChangeType::Modified,
                severity: Severity::Additive,
                message: format!("extension {key} changed"),
                old_value: Some(old.clone()),
                new_value: Some(new.clone()),
            }],
            _ => vec![],
        }
    }
}

/// Registry of extension processors. The comparator uses this to delegate
/// extension diffing to the appropriate handler.
pub struct ExtensionRegistry {
    processors: Vec<Box<dyn ExtensionProcessor>>,
}

impl ExtensionRegistry {
    /// Create a new registry with no processors.
    pub fn new() -> Self {
        ExtensionRegistry {
            processors: Vec::new(),
        }
    }

    /// Create a registry with the default built-in processors.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(extensible_enum::ExtensibleEnumProcessor));
        registry
    }

    /// Register a custom extension processor.
    pub fn register(&mut self, processor: Box<dyn ExtensionProcessor>) {
        self.processors.push(processor);
    }

    /// Process a single extension key. Returns changes produced by the matching
    /// processor, or the default processor if no specific one is registered.
    pub fn process(
        &self,
        path: &str,
        key: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Vec<Change> {
        // Find a dedicated processor for this key
        for processor in &self.processors {
            if processor.key() == key {
                return processor.process(path, old_value, new_value);
            }
        }

        // Fall back to default processing
        DefaultExtensionProcessor::process_extension(path, key, old_value, new_value)
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
