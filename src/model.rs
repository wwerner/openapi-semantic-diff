// src/model.rs — Internal data types for semantic diffing

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Internal OpenAPI representation (version-agnostic)
// ---------------------------------------------------------------------------

/// Version-agnostic internal representation of an OpenAPI spec.
/// Parsers for 3.0.x and 3.1.x both produce this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalSpec {
    pub openapi_version: String,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: IndexMap<String, PathItem>,
    pub components: Components,
    pub security: Vec<SecurityRequirement>,
    pub tags: Vec<Tag>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Server {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    pub operations: IndexMap<HttpMethod, Operation>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
    Trace,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Options => "OPTIONS",
            Self::Head => "HEAD",
            Self::Trace => "TRACE",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub operation_id: Option<String>,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: IndexMap<String, Response>,
    pub security: Vec<SecurityRequirement>,
    pub deprecated: bool,
    pub tags: Vec<String>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: Option<Schema>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Path,
    Header,
    Cookie,
}

impl fmt::Display for ParameterLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Query => "query",
            Self::Path => "path",
            Self::Header => "header",
            Self::Cookie => "cookie",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub required: bool,
    pub description: Option<String>,
    pub content: IndexMap<String, MediaType>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: Option<String>,
    pub content: IndexMap<String, MediaType>,
    pub headers: IndexMap<String, Header>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub required: bool,
    pub schema: Option<Schema>,
    pub description: Option<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub schema_type: Option<SchemaType>,
    pub format: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub nullable: bool,
    pub required: Vec<String>,
    pub properties: IndexMap<String, Schema>,
    pub items: Option<Box<Schema>>,
    pub enum_values: Vec<serde_json::Value>,
    pub all_of: Vec<Schema>,
    pub one_of: Vec<Schema>,
    pub any_of: Vec<Schema>,
    pub default: Option<serde_json::Value>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub min_length: Option<u64>,
    pub max_length: Option<u64>,
    pub pattern: Option<String>,
    pub read_only: bool,
    pub write_only: bool,
    pub deprecated: bool,
    pub extensions: IndexMap<String, serde_json::Value>,
    /// Sentinel for cyclic `$ref`s — if `Some`, this schema is a back-reference.
    pub cyclic_ref: Option<String>,
}

impl Schema {
    /// Create a sentinel schema representing a cyclic `$ref`.
    pub fn cyclic(name: impl Into<String>) -> Self {
        Schema {
            cyclic_ref: Some(name.into()),
            ..Schema::default()
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Schema {
            schema_type: None,
            format: None,
            title: None,
            description: None,
            nullable: false,
            required: Vec::new(),
            properties: IndexMap::new(),
            items: None,
            enum_values: Vec::new(),
            all_of: Vec::new(),
            one_of: Vec::new(),
            any_of: Vec::new(),
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            read_only: false,
            write_only: false,
            deprecated: false,
            extensions: IndexMap::new(),
            cyclic_ref: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}

impl fmt::Display for SchemaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
        };
        write!(f, "{}", s)
    }
}

/// Simple alias — keys are scheme names, values are lists of scopes.
pub type SecurityRequirement = IndexMap<String, Vec<String>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: IndexMap<String, Schema>,
    pub security_schemes: IndexMap<String, SecurityScheme>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

impl Default for Components {
    fn default() -> Self {
        Components {
            schemas: IndexMap::new(),
            security_schemes: IndexMap::new(),
            extensions: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    pub scheme_type: SecuritySchemeType,
    pub description: Option<String>,
    pub name: Option<String>,
    pub location: Option<String>,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub flows: Option<OAuthFlows>,
    pub open_id_connect_url: Option<String>,
    pub extensions: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SecuritySchemeType {
    ApiKey,
    Http,
    OAuth2,
    OpenIdConnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlows {
    pub implicit: Option<OAuthFlow>,
    pub password: Option<OAuthFlow>,
    pub client_credentials: Option<OAuthFlow>,
    pub authorization_code: Option<OAuthFlow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlow {
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub refresh_url: Option<String>,
    pub scopes: IndexMap<String, String>,
}

// ---------------------------------------------------------------------------
// Diff report types
// ---------------------------------------------------------------------------

/// The top-level diff report produced by the comparator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub info_changes: Vec<Change>,
    pub server_changes: Vec<Change>,
    pub path_changes: Vec<Change>,
    pub schema_changes: Vec<Change>,
    pub security_scheme_changes: Vec<Change>,
    pub tag_changes: Vec<Change>,
    pub extension_changes: Vec<Change>,
}

impl DiffReport {
    pub fn is_empty(&self) -> bool {
        self.info_changes.is_empty()
            && self.server_changes.is_empty()
            && self.path_changes.is_empty()
            && self.schema_changes.is_empty()
            && self.security_scheme_changes.is_empty()
            && self.tag_changes.is_empty()
            && self.extension_changes.is_empty()
    }

    /// Return all changes across every category.
    pub fn all_changes(&self) -> Vec<&Change> {
        let mut out = Vec::new();
        for changes in [
            &self.info_changes,
            &self.server_changes,
            &self.path_changes,
            &self.schema_changes,
            &self.security_scheme_changes,
            &self.tag_changes,
            &self.extension_changes,
        ] {
            out.extend(changes.iter());
        }
        out
    }

    /// Return the highest severity found, or `None` if no changes.
    pub fn max_severity(&self) -> Option<Severity> {
        self.all_changes().iter().map(|c| c.severity).max()
    }

    /// Filter changes to only those at or above the given severity.
    pub fn filtered(&self, min_severity: Severity) -> DiffReport {
        let filter = |changes: &[Change]| -> Vec<Change> {
            changes
                .iter()
                .filter(|c| c.severity >= min_severity)
                .cloned()
                .collect()
        };
        DiffReport {
            info_changes: filter(&self.info_changes),
            server_changes: filter(&self.server_changes),
            path_changes: filter(&self.path_changes),
            schema_changes: filter(&self.schema_changes),
            security_scheme_changes: filter(&self.security_scheme_changes),
            tag_changes: filter(&self.tag_changes),
            extension_changes: filter(&self.extension_changes),
        }
    }
}

/// A single semantic change detected between old and new specs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub path: String,
    pub change_type: ChangeType,
    pub severity: Severity,
    pub message: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
    Deprecated,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
            Self::Deprecated => "deprecated",
        };
        write!(f, "{}", s)
    }
}

/// Severity tier. Ordered so `Breaking > Deprecated > Additive`.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// New capabilities added; no consumer impact.
    Additive,
    /// Something has been deprecated; not broken yet.
    Deprecated,
    /// Consumers will break if they don't adapt.
    Breaking,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Additive => "additive",
            Self::Deprecated => "deprecated",
            Self::Breaking => "breaking",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "additive" => Ok(Severity::Additive),
            "deprecated" => Ok(Severity::Deprecated),
            "breaking" => Ok(Severity::Breaking),
            _ => Err(format!("unknown severity: {s}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Grouped / human-readable output types
// ---------------------------------------------------------------------------

/// The two top-level sections in human-readable output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedReport {
    pub total_changes: usize,
    pub max_severity: Option<String>,
    /// API path changes, grouped by route → verb → property, with inlined
    /// schema changes merged in.
    pub paths: Vec<PathGroup>,
    /// Non-endpoint changes (info, servers, schemas, security schemes, tags,
    /// extensions), grouped by category/item.
    pub metadata: Vec<MetadataGroup>,
}

/// One API route (e.g. `/pets` or `/pets/{petId}`) with all its verb groups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathGroup {
    /// The route string, e.g. `/pets/{petId}`.
    pub route: String,
    pub verb_groups: Vec<VerbGroup>,
}

/// One HTTP verb within a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerbGroup {
    /// The HTTP verb string, e.g. `GET`.
    pub verb: String,
    pub property_groups: Vec<PropertyGroup>,
}

/// One logical property / sub-path within a verb, e.g.
/// `parameters.limit.query` or `responses.200.content.application/json.schema.items.properties.id.type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyGroup {
    /// The sub-path relative to `paths.<route>.<VERB>`, e.g. `requestBody`,
    /// `parameters.limit.query`, `responses.200`.
    pub property: String,
    /// Changes ordered Added → Removed → Modified → Deprecated.
    pub changes: Vec<ChangeEntry>,
}

/// One change rendered for template consumption (flat, serialisable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    pub path: String,
    pub change_type: String,
    pub severity: String,
    pub message: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

impl From<&Change> for ChangeEntry {
    fn from(c: &Change) -> Self {
        ChangeEntry {
            path: c.path.clone(),
            change_type: c.change_type.to_string(),
            severity: c.severity.to_string(),
            message: c.message.clone(),
            old_value: c.old_value.clone(),
            new_value: c.new_value.clone(),
        }
    }
}

/// A category of non-endpoint metadata changes, e.g. "Info", "Servers",
/// "Schemas > Pet", "Security Schemes", "Tags".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataGroup {
    /// Human-readable label for the category, e.g. `Info`, `Schemas > Pet`.
    pub label: String,
    pub property_groups: Vec<PropertyGroup>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum OsdError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported OpenAPI version: {0}")]
    UnsupportedVersion(String),

    #[error("$ref resolution error: {0}")]
    RefResolution(String),

    #[error("Remote $ref not supported: {0}")]
    RemoteRef(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("{0}")]
    Other(String),
}
