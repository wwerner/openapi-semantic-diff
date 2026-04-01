// src/parser/openapi31.rs — OpenAPI 3.1.x parser using the `oas3` crate
//
// Converts from `oas3::spec::*` types to our `model::InternalSpec`.
// All `$ref`s have already been inlined by `ref_resolver::preprocess`.

use crate::model::*;
use crate::ref_resolver;
use indexmap::IndexMap;
use std::path::Path;

/// Parse an OpenAPI 3.1.x spec string into an `InternalSpec`.
pub fn parse(content: &str, source_path: Option<&Path>) -> Result<InternalSpec, OsdError> {
    // Pre-process: resolve all $refs at the Value level
    let value = ref_resolver::preprocess(content, source_path)?;

    // Deserialize the fully-inlined Value into the oas3 types
    let spec: oas3::spec::Spec = serde_json::from_value(value)
        .map_err(|e| OsdError::Other(format!("OpenAPI 3.1 parse error: {e}")))?;

    convert(spec)
}

fn convert(spec: oas3::spec::Spec) -> Result<InternalSpec, OsdError> {
    Ok(InternalSpec {
        openapi_version: spec.openapi.clone(),
        info: convert_info(&spec.info),
        servers: spec.servers.iter().map(convert_server).collect(),
        paths: convert_paths(&spec),
        components: convert_components(spec.components.as_ref()),
        security: spec
            .security
            .iter()
            .map(convert_security_requirement)
            .collect(),
        tags: spec.tags.iter().map(convert_tag).collect(),
        // oas3 strips the "x-" prefix from extension keys — add it back
        extensions: spec
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    })
}

/// Ensure extension keys have the `x-` prefix (oas3 strips it).
fn ensure_x_prefix(key: &str) -> String {
    if key.starts_with("x-") {
        key.to_string()
    } else {
        format!("x-{key}")
    }
}

fn convert_info(info: &oas3::spec::Info) -> Info {
    Info {
        title: info.title.clone(),
        version: info.version.clone(),
        description: info.description.clone(),
    }
}

fn convert_server(server: &oas3::spec::Server) -> Server {
    Server {
        url: server.url.clone(),
        description: server.description.clone(),
    }
}

fn convert_paths(spec: &oas3::spec::Spec) -> IndexMap<String, PathItem> {
    let mut result = IndexMap::new();
    if let Some(paths) = &spec.paths {
        for (path, item) in paths {
            result.insert(path.clone(), convert_path_item(item));
        }
    }
    result
}

fn convert_path_item(item: &oas3::spec::PathItem) -> PathItem {
    let mut operations = IndexMap::new();

    let method_ops: &[(HttpMethod, &Option<oas3::spec::Operation>)] = &[
        (HttpMethod::Get, &item.get),
        (HttpMethod::Post, &item.post),
        (HttpMethod::Put, &item.put),
        (HttpMethod::Delete, &item.delete),
        (HttpMethod::Patch, &item.patch),
        (HttpMethod::Options, &item.options),
        (HttpMethod::Head, &item.head),
        (HttpMethod::Trace, &item.trace),
    ];

    for (method, op) in method_ops {
        if let Some(op) = op {
            operations.insert(*method, convert_operation(op, &item.parameters));
        }
    }

    PathItem {
        operations,
        extensions: item
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    }
}

fn convert_operation(
    op: &oas3::spec::Operation,
    path_params: &[oas3::spec::ObjectOrReference<oas3::spec::Parameter>],
) -> Operation {
    // Merge path-level and operation-level parameters
    let mut params: Vec<Parameter> = path_params
        .iter()
        .filter_map(|p| match p {
            oas3::spec::ObjectOrReference::Object(param) => Some(convert_parameter(param)),
            _ => None,
        })
        .collect();

    for p in &op.parameters {
        if let oas3::spec::ObjectOrReference::Object(param) = p {
            let converted = convert_parameter(param);
            params.retain(|existing| {
                !(existing.name == converted.name && existing.location == converted.location)
            });
            params.push(converted);
        }
    }

    Operation {
        summary: op.summary.clone(),
        description: op.description.clone(),
        operation_id: op.operation_id.clone(),
        parameters: params,
        request_body: op.request_body.as_ref().and_then(|rb| match rb {
            oas3::spec::ObjectOrReference::Object(body) => Some(convert_request_body(body)),
            _ => None,
        }),
        responses: convert_responses(&op.responses),
        security: op
            .security
            .iter()
            .map(convert_security_requirement)
            .collect(),
        deprecated: op.deprecated.unwrap_or(false),
        tags: op.tags.clone(),
        extensions: op
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    }
}

fn convert_parameter(param: &oas3::spec::Parameter) -> Parameter {
    let location = match param.location {
        oas3::spec::ParameterIn::Query => ParameterLocation::Query,
        oas3::spec::ParameterIn::Header => ParameterLocation::Header,
        oas3::spec::ParameterIn::Path => ParameterLocation::Path,
        oas3::spec::ParameterIn::Cookie => ParameterLocation::Cookie,
    };

    let schema = param.schema.as_ref().and_then(|s| match s {
        oas3::spec::ObjectOrReference::Object(obj) => Some(convert_object_schema(obj)),
        _ => None,
    });

    Parameter {
        name: param.name.clone(),
        location,
        required: param.required.unwrap_or(false),
        schema,
        description: param.description.clone(),
        deprecated: param.deprecated.unwrap_or(false),
        extensions: param
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    }
}

fn convert_request_body(rb: &oas3::spec::RequestBody) -> RequestBody {
    RequestBody {
        required: rb.required.unwrap_or(false),
        description: rb.description.clone(),
        content: rb
            .content
            .iter()
            .map(|(k, v)| (k.clone(), convert_media_type(v)))
            .collect(),
        extensions: IndexMap::new(), // oas3 doesn't expose extensions on RequestBody
    }
}

fn convert_media_type(mt: &oas3::spec::MediaType) -> MediaType {
    MediaType {
        schema: mt.schema.as_ref().and_then(|s| match s {
            oas3::spec::ObjectOrReference::Object(obj) => Some(convert_object_schema(obj)),
            _ => None,
        }),
    }
}

fn convert_schema_enum(schema: &oas3::spec::Schema) -> Schema {
    match schema {
        oas3::spec::Schema::Boolean(_) => {
            // Boolean schemas (true/false) — we represent as a minimal schema
            Schema::default()
        }
        oas3::spec::Schema::Object(obj_or_ref) => match obj_or_ref.as_ref() {
            oas3::spec::ObjectOrReference::Object(obj) => convert_object_schema(obj),
            oas3::spec::ObjectOrReference::Ref { .. } => {
                // Should not appear after ref resolution
                Schema::default()
            }
        },
    }
}

fn convert_object_schema(obj: &oas3::spec::ObjectSchema) -> Schema {
    // Check for cyclic ref marker
    if let Some(cyclic_name) = obj.extensions.get("osd-cyclic-ref") {
        // oas3 strips "x-" prefix, so our "x-osd-cyclic-ref" becomes "osd-cyclic-ref"
        if let Some(name) = cyclic_name.as_str() {
            return Schema::cyclic(name);
        }
    }

    let schema_type = obj.schema_type.as_ref().and_then(|ts| match ts {
        oas3::spec::SchemaTypeSet::Single(t) => convert_type_single(t),
        oas3::spec::SchemaTypeSet::Multiple(types) => {
            // For nullable types like ["string", "null"], pick the non-null type
            types
                .iter()
                .find(|t| !matches!(t, oas3::spec::SchemaType::Null))
                .and_then(convert_type_single)
        }
    });

    let nullable = obj
        .schema_type
        .as_ref()
        .map(|ts| match ts {
            oas3::spec::SchemaTypeSet::Multiple(types) => types
                .iter()
                .any(|t| matches!(t, oas3::spec::SchemaType::Null)),
            _ => false,
        })
        .unwrap_or(false);

    Schema {
        schema_type,
        format: obj.format.clone(),
        title: obj.title.clone(),
        description: obj.description.clone(),
        nullable,
        required: obj.required.clone(),
        properties: obj
            .properties
            .iter()
            .map(|(k, v)| {
                let schema = match v {
                    oas3::spec::ObjectOrReference::Object(o) => convert_object_schema(o),
                    _ => Schema::default(),
                };
                (k.clone(), schema)
            })
            .collect(),
        items: obj.items.as_ref().map(|s| Box::new(convert_schema_enum(s))),
        enum_values: obj.enum_values.clone(),
        all_of: obj
            .all_of
            .iter()
            .filter_map(|s| match s {
                oas3::spec::ObjectOrReference::Object(o) => Some(convert_object_schema(o)),
                _ => None,
            })
            .collect(),
        one_of: obj
            .one_of
            .iter()
            .filter_map(|s| match s {
                oas3::spec::ObjectOrReference::Object(o) => Some(convert_object_schema(o)),
                _ => None,
            })
            .collect(),
        any_of: obj
            .any_of
            .iter()
            .filter_map(|s| match s {
                oas3::spec::ObjectOrReference::Object(o) => Some(convert_object_schema(o)),
                _ => None,
            })
            .collect(),
        default: obj.default.clone(),
        minimum: obj.minimum.as_ref().and_then(|n| n.as_f64()),
        maximum: obj.maximum.as_ref().and_then(|n| n.as_f64()),
        min_length: obj.min_length,
        max_length: obj.max_length,
        pattern: obj.pattern.clone(),
        read_only: obj.read_only.unwrap_or(false),
        write_only: obj.write_only.unwrap_or(false),
        deprecated: obj.deprecated.unwrap_or(false),
        extensions: obj
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
        cyclic_ref: None,
    }
}

fn convert_type_single(t: &oas3::spec::SchemaType) -> Option<SchemaType> {
    match t {
        oas3::spec::SchemaType::String => Some(SchemaType::String),
        oas3::spec::SchemaType::Number => Some(SchemaType::Number),
        oas3::spec::SchemaType::Integer => Some(SchemaType::Integer),
        oas3::spec::SchemaType::Boolean => Some(SchemaType::Boolean),
        oas3::spec::SchemaType::Array => Some(SchemaType::Array),
        oas3::spec::SchemaType::Object => Some(SchemaType::Object),
        oas3::spec::SchemaType::Null => None,
    }
}

fn convert_responses(
    responses: &Option<
        std::collections::BTreeMap<String, oas3::spec::ObjectOrReference<oas3::spec::Response>>,
    >,
) -> IndexMap<String, Response> {
    let mut result = IndexMap::new();
    if let Some(map) = responses {
        for (code, ref_or_resp) in map {
            if let oas3::spec::ObjectOrReference::Object(resp) = ref_or_resp {
                result.insert(code.clone(), convert_response(resp));
            }
        }
    }
    result
}

fn convert_response(resp: &oas3::spec::Response) -> Response {
    Response {
        description: resp.description.clone(),
        content: resp
            .content
            .iter()
            .map(|(k, v)| (k.clone(), convert_media_type(v)))
            .collect(),
        headers: resp
            .headers
            .iter()
            .filter_map(|(k, v)| match v {
                oas3::spec::ObjectOrReference::Object(h) => Some((k.clone(), convert_header(h))),
                _ => None,
            })
            .collect(),
        extensions: resp
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    }
}

fn convert_header(header: &oas3::spec::Header) -> Header {
    let schema = header.schema.as_ref().and_then(|s| match s {
        oas3::spec::ObjectOrReference::Object(obj) => Some(convert_object_schema(obj)),
        _ => None,
    });

    Header {
        required: header.required.unwrap_or(false),
        schema,
        description: header.description.clone(),
        deprecated: header.deprecated.unwrap_or(false),
    }
}

fn convert_components(components: Option<&oas3::spec::Components>) -> Components {
    let Some(c) = components else {
        return Components::default();
    };

    Components {
        schemas: c
            .schemas
            .iter()
            .filter_map(|(k, v)| match v {
                oas3::spec::ObjectOrReference::Object(obj) => {
                    Some((k.clone(), convert_object_schema(obj)))
                }
                _ => None,
            })
            .collect(),
        security_schemes: c
            .security_schemes
            .iter()
            .filter_map(|(k, v)| match v {
                oas3::spec::ObjectOrReference::Object(s) => {
                    Some((k.clone(), convert_security_scheme(s)))
                }
                _ => None,
            })
            .collect(),
        extensions: c
            .extensions
            .iter()
            .map(|(k, v)| (ensure_x_prefix(k), v.clone()))
            .collect(),
    }
}

fn convert_security_scheme(scheme: &oas3::spec::SecurityScheme) -> SecurityScheme {
    match scheme {
        oas3::spec::SecurityScheme::ApiKey {
            description,
            name,
            location,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::ApiKey,
            description: description.clone(),
            name: Some(name.clone()),
            location: Some(location.clone()),
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
            extensions: IndexMap::new(),
        },
        oas3::spec::SecurityScheme::Http {
            description,
            scheme,
            bearer_format,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::Http,
            description: description.clone(),
            name: None,
            location: None,
            scheme: Some(scheme.clone()),
            bearer_format: bearer_format.clone(),
            flows: None,
            open_id_connect_url: None,
            extensions: IndexMap::new(),
        },
        oas3::spec::SecurityScheme::OAuth2 { description, flows } => SecurityScheme {
            scheme_type: SecuritySchemeType::OAuth2,
            description: description.clone(),
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: Some(convert_oauth_flows(flows)),
            open_id_connect_url: None,
            extensions: IndexMap::new(),
        },
        oas3::spec::SecurityScheme::OpenIdConnect {
            description,
            open_id_connect_url,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::OpenIdConnect,
            description: description.clone(),
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: Some(open_id_connect_url.to_string()),
            extensions: IndexMap::new(),
        },
        oas3::spec::SecurityScheme::MutualTls { description } => SecurityScheme {
            scheme_type: SecuritySchemeType::Http, // Best approximation
            description: description.clone(),
            name: None,
            location: None,
            scheme: Some("mutualTLS".to_string()),
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
            extensions: IndexMap::new(),
        },
    }
}

fn convert_oauth_flows(flows: &oas3::spec::Flows) -> OAuthFlows {
    OAuthFlows {
        implicit: flows.implicit.as_ref().map(|f| OAuthFlow {
            authorization_url: Some(f.authorization_url.to_string()),
            token_url: None,
            refresh_url: f.refresh_url.as_ref().map(|u| u.to_string()),
            scopes: f
                .scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }),
        password: flows.password.as_ref().map(|f| OAuthFlow {
            authorization_url: None,
            token_url: Some(f.token_url.to_string()),
            refresh_url: f.refresh_url.as_ref().map(|u| u.to_string()),
            scopes: f
                .scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }),
        client_credentials: flows.client_credentials.as_ref().map(|f| OAuthFlow {
            authorization_url: None,
            token_url: Some(f.token_url.to_string()),
            refresh_url: f.refresh_url.as_ref().map(|u| u.to_string()),
            scopes: f
                .scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }),
        authorization_code: flows.authorization_code.as_ref().map(|f| OAuthFlow {
            authorization_url: Some(f.authorization_url.to_string()),
            token_url: Some(f.token_url.to_string()),
            refresh_url: f.refresh_url.as_ref().map(|u| u.to_string()),
            scopes: f
                .scopes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }),
    }
}

fn convert_security_requirement(req: &oas3::spec::SecurityRequirement) -> SecurityRequirement {
    req.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn convert_tag(tag: &oas3::spec::Tag) -> Tag {
    Tag {
        name: tag.name.clone(),
        description: tag.description.clone(),
    }
}
