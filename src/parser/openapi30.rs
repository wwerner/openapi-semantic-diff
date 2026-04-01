// src/parser/openapi30.rs — OpenAPI 3.0.x parser using the `openapiv3` crate
//
// Converts from `openapiv3::OpenAPI` types to our `model::InternalSpec`.
// All `$ref`s have already been inlined by `ref_resolver::preprocess`.

use crate::model::*;
use crate::ref_resolver;
use indexmap::IndexMap;
use std::path::Path;

/// Parse an OpenAPI 3.0.x spec string into an `InternalSpec`.
pub fn parse(content: &str, source_path: Option<&Path>) -> Result<InternalSpec, OsdError> {
    // Pre-process: resolve all $refs at the Value level
    let value = ref_resolver::preprocess(content, source_path)?;

    // Now deserialize the fully-inlined Value into the openapiv3 types
    let spec: openapiv3::OpenAPI = serde_json::from_value(value)
        .map_err(|e| OsdError::Other(format!("OpenAPI 3.0 parse error: {e}")))?;

    convert(spec)
}

fn convert(spec: openapiv3::OpenAPI) -> Result<InternalSpec, OsdError> {
    Ok(InternalSpec {
        openapi_version: spec.openapi.clone(),
        info: convert_info(&spec.info),
        servers: spec.servers.iter().map(convert_server).collect(),
        paths: convert_paths(&spec.paths),
        components: convert_components(spec.components.as_ref()),
        security: spec
            .security
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(convert_security_requirement)
            .collect(),
        tags: spec.tags.iter().map(convert_tag).collect(),
        extensions: spec.extensions,
    })
}

fn convert_info(info: &openapiv3::Info) -> Info {
    Info {
        title: info.title.clone(),
        version: info.version.clone(),
        description: info.description.clone(),
    }
}

fn convert_server(server: &openapiv3::Server) -> Server {
    Server {
        url: server.url.clone(),
        description: server.description.clone(),
    }
}

fn convert_paths(paths: &openapiv3::Paths) -> IndexMap<String, PathItem> {
    let mut result = IndexMap::new();
    for (path, ref_or_item) in &paths.paths {
        if let Some(item) = ref_or_item.as_item() {
            result.insert(path.clone(), convert_path_item(item));
        }
        // ReferenceOr::Reference should not appear after ref resolution
    }
    result
}

fn convert_path_item(item: &openapiv3::PathItem) -> PathItem {
    let mut operations = IndexMap::new();

    let method_ops: &[(HttpMethod, &Option<openapiv3::Operation>)] = &[
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
        extensions: item.extensions.clone(),
    }
}

fn convert_operation(
    op: &openapiv3::Operation,
    path_params: &[openapiv3::ReferenceOr<openapiv3::Parameter>],
) -> Operation {
    // Merge path-level and operation-level parameters.
    // Operation-level params override path-level params with the same (name, in).
    let mut params: Vec<Parameter> = path_params
        .iter()
        .filter_map(|p| p.as_item())
        .map(convert_parameter)
        .collect();

    for p in &op.parameters {
        if let Some(param) = p.as_item() {
            let converted = convert_parameter(param);
            // Remove any path-level param with the same (name, location)
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
        request_body: op
            .request_body
            .as_ref()
            .and_then(|rb| rb.as_item().map(convert_request_body)),
        responses: convert_responses(&op.responses),
        security: op
            .security
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(convert_security_requirement)
            .collect(),
        deprecated: op.deprecated,
        tags: op.tags.clone(),
        extensions: op.extensions.clone(),
    }
}

fn convert_parameter(param: &openapiv3::Parameter) -> Parameter {
    let data = param.parameter_data_ref();
    let location = match param {
        openapiv3::Parameter::Query { .. } => ParameterLocation::Query,
        openapiv3::Parameter::Header { .. } => ParameterLocation::Header,
        openapiv3::Parameter::Path { .. } => ParameterLocation::Path,
        openapiv3::Parameter::Cookie { .. } => ParameterLocation::Cookie,
    };

    let schema = match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(s) => s.as_item().map(convert_schema),
        openapiv3::ParameterSchemaOrContent::Content(_) => None,
    };

    Parameter {
        name: data.name.clone(),
        location,
        required: data.required,
        schema,
        description: data.description.clone(),
        deprecated: data.deprecated.unwrap_or(false),
        extensions: data.extensions.clone(),
    }
}

fn convert_request_body(rb: &openapiv3::RequestBody) -> RequestBody {
    RequestBody {
        required: rb.required,
        description: rb.description.clone(),
        content: rb
            .content
            .iter()
            .map(|(k, v)| (k.clone(), convert_media_type(v)))
            .collect(),
        extensions: rb.extensions.clone(),
    }
}

fn convert_media_type(mt: &openapiv3::MediaType) -> MediaType {
    MediaType {
        schema: mt
            .schema
            .as_ref()
            .and_then(|s| s.as_item().map(convert_schema)),
    }
}

fn convert_responses(responses: &openapiv3::Responses) -> IndexMap<String, Response> {
    let mut result = IndexMap::new();

    if let Some(default) = &responses.default {
        if let Some(resp) = default.as_item() {
            result.insert("default".to_string(), convert_response(resp));
        }
    }

    for (code, ref_or_resp) in &responses.responses {
        if let Some(resp) = ref_or_resp.as_item() {
            let key = match code {
                openapiv3::StatusCode::Code(n) => n.to_string(),
                openapiv3::StatusCode::Range(n) => format!("{n}XX"),
            };
            result.insert(key, convert_response(resp));
        }
    }

    result
}

fn convert_response(resp: &openapiv3::Response) -> Response {
    Response {
        description: Some(resp.description.clone()),
        content: resp
            .content
            .iter()
            .map(|(k, v)| (k.clone(), convert_media_type(v)))
            .collect(),
        headers: resp
            .headers
            .iter()
            .filter_map(|(k, v)| v.as_item().map(|h| (k.clone(), convert_header(h))))
            .collect(),
        extensions: resp.extensions.clone(),
    }
}

fn convert_header(header: &openapiv3::Header) -> Header {
    let schema = match &header.format {
        openapiv3::ParameterSchemaOrContent::Schema(s) => s.as_item().map(convert_schema),
        openapiv3::ParameterSchemaOrContent::Content(_) => None,
    };

    Header {
        required: header.required,
        schema,
        description: header.description.clone(),
        deprecated: header.deprecated.unwrap_or(false),
    }
}

fn convert_schema(schema: &openapiv3::Schema) -> Schema {
    let data = &schema.schema_data;

    // Check for cyclic ref marker (set by our ref resolver)
    if let Some(cyclic_name) = data.extensions.get("x-osd-cyclic-ref") {
        if let Some(name) = cyclic_name.as_str() {
            return Schema::cyclic(name);
        }
    }

    let mut result = Schema {
        title: data.title.clone(),
        description: data.description.clone(),
        nullable: data.nullable,
        read_only: data.read_only,
        write_only: data.write_only,
        deprecated: data.deprecated,
        default: data.default.clone(),
        extensions: data.extensions.clone(),
        ..Schema::default()
    };

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(t) => convert_type(t, &mut result),
        openapiv3::SchemaKind::OneOf { one_of } => {
            result.one_of = one_of
                .iter()
                .filter_map(|s| s.as_item().map(convert_schema))
                .collect();
        }
        openapiv3::SchemaKind::AllOf { all_of } => {
            result.all_of = all_of
                .iter()
                .filter_map(|s| s.as_item().map(convert_schema))
                .collect();
        }
        openapiv3::SchemaKind::AnyOf { any_of } => {
            result.any_of = any_of
                .iter()
                .filter_map(|s| s.as_item().map(convert_schema))
                .collect();
        }
        openapiv3::SchemaKind::Not { .. } => {
            // We don't model `not` in our internal representation yet
        }
        openapiv3::SchemaKind::Any(any) => {
            convert_any_schema(any, &mut result);
        }
    }

    result
}

fn convert_type(t: &openapiv3::Type, result: &mut Schema) {
    match t {
        openapiv3::Type::String(s) => {
            result.schema_type = Some(SchemaType::String);
            result.format = string_format_to_string(&s.format);
            result.pattern = s.pattern.clone();
            result.min_length = s.min_length.map(|v| v as u64);
            result.max_length = s.max_length.map(|v| v as u64);
            result.enum_values = s
                .enumeration
                .iter()
                .filter_map(|v| v.as_ref())
                .map(|v| serde_json::Value::String(v.clone()))
                .collect();
        }
        openapiv3::Type::Number(n) => {
            result.schema_type = Some(SchemaType::Number);
            result.format = number_format_to_string(&n.format);
            result.minimum = n.minimum;
            result.maximum = n.maximum;
            result.enum_values = n
                .enumeration
                .iter()
                .filter_map(|v| v.as_ref())
                .map(|v| serde_json::json!(v))
                .collect();
        }
        openapiv3::Type::Integer(i) => {
            result.schema_type = Some(SchemaType::Integer);
            result.format = integer_format_to_string(&i.format);
            result.minimum = i.minimum.map(|v| v as f64);
            result.maximum = i.maximum.map(|v| v as f64);
            result.enum_values = i
                .enumeration
                .iter()
                .filter_map(|v| v.as_ref())
                .map(|v| serde_json::json!(v))
                .collect();
        }
        openapiv3::Type::Boolean(b) => {
            result.schema_type = Some(SchemaType::Boolean);
            result.enum_values = b
                .enumeration
                .iter()
                .filter_map(|v| v.as_ref())
                .map(|v| serde_json::json!(v))
                .collect();
        }
        openapiv3::Type::Object(obj) => {
            result.schema_type = Some(SchemaType::Object);
            result.required = obj.required.clone();
            result.properties = obj
                .properties
                .iter()
                .filter_map(|(k, v)| v.as_item().map(|s| (k.clone(), convert_schema(s))))
                .collect();
        }
        openapiv3::Type::Array(arr) => {
            result.schema_type = Some(SchemaType::Array);
            result.items = arr
                .items
                .as_ref()
                .and_then(|i| i.as_item().map(|s| Box::new(convert_schema(s))));
        }
    }
}

fn convert_any_schema(any: &openapiv3::AnySchema, result: &mut Schema) {
    // AnySchema is a catch-all for schemas that mix type with composition keywords.
    // We extract what we can.
    if let Some(ref t) = any.typ {
        result.schema_type = match t.as_str() {
            "string" => Some(SchemaType::String),
            "number" => Some(SchemaType::Number),
            "integer" => Some(SchemaType::Integer),
            "boolean" => Some(SchemaType::Boolean),
            "array" => Some(SchemaType::Array),
            "object" => Some(SchemaType::Object),
            _ => None,
        };
    }
    if let Some(ref fmt) = any.format {
        result.format = Some(fmt.clone());
    }
    if let Some(ref pattern) = any.pattern {
        result.pattern = Some(pattern.clone());
    }
    result.enum_values = any.enumeration.clone();
    result.required = any.required.clone();
    result.properties = any
        .properties
        .iter()
        .filter_map(|(k, v)| v.as_item().map(|s| (k.clone(), convert_schema(s))))
        .collect();
    result.items = any
        .items
        .as_ref()
        .and_then(|i| i.as_item().map(|s| Box::new(convert_schema(s))));
    result.one_of = any
        .one_of
        .iter()
        .filter_map(|s| s.as_item().map(convert_schema))
        .collect();
    result.all_of = any
        .all_of
        .iter()
        .filter_map(|s| s.as_item().map(convert_schema))
        .collect();
    result.any_of = any
        .any_of
        .iter()
        .filter_map(|s| s.as_item().map(convert_schema))
        .collect();
}

fn string_format_to_string(
    fmt: &openapiv3::VariantOrUnknownOrEmpty<openapiv3::StringFormat>,
) -> Option<String> {
    match fmt {
        openapiv3::VariantOrUnknownOrEmpty::Item(v) => {
            let s = match v {
                openapiv3::StringFormat::Date => "date",
                openapiv3::StringFormat::DateTime => "date-time",
                openapiv3::StringFormat::Password => "password",
                openapiv3::StringFormat::Byte => "byte",
                openapiv3::StringFormat::Binary => "binary",
            };
            Some(s.to_string())
        }
        openapiv3::VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
        openapiv3::VariantOrUnknownOrEmpty::Empty => None,
    }
}

fn number_format_to_string(
    fmt: &openapiv3::VariantOrUnknownOrEmpty<openapiv3::NumberFormat>,
) -> Option<String> {
    match fmt {
        openapiv3::VariantOrUnknownOrEmpty::Item(v) => {
            let s = match v {
                openapiv3::NumberFormat::Float => "float",
                openapiv3::NumberFormat::Double => "double",
            };
            Some(s.to_string())
        }
        openapiv3::VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
        openapiv3::VariantOrUnknownOrEmpty::Empty => None,
    }
}

fn integer_format_to_string(
    fmt: &openapiv3::VariantOrUnknownOrEmpty<openapiv3::IntegerFormat>,
) -> Option<String> {
    match fmt {
        openapiv3::VariantOrUnknownOrEmpty::Item(v) => {
            let s = match v {
                openapiv3::IntegerFormat::Int32 => "int32",
                openapiv3::IntegerFormat::Int64 => "int64",
            };
            Some(s.to_string())
        }
        openapiv3::VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
        openapiv3::VariantOrUnknownOrEmpty::Empty => None,
    }
}

fn convert_components(components: Option<&openapiv3::Components>) -> Components {
    let Some(c) = components else {
        return Components::default();
    };

    Components {
        schemas: c
            .schemas
            .iter()
            .filter_map(|(k, v)| v.as_item().map(|s| (k.clone(), convert_schema(s))))
            .collect(),
        security_schemes: c
            .security_schemes
            .iter()
            .filter_map(|(k, v)| v.as_item().map(|s| (k.clone(), convert_security_scheme(s))))
            .collect(),
        extensions: c.extensions.clone(),
    }
}

fn convert_security_scheme(scheme: &openapiv3::SecurityScheme) -> SecurityScheme {
    match scheme {
        openapiv3::SecurityScheme::APIKey {
            location,
            name,
            description,
            extensions,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::ApiKey,
            description: description.clone(),
            name: Some(name.clone()),
            location: Some(match location {
                openapiv3::APIKeyLocation::Query => "query".to_string(),
                openapiv3::APIKeyLocation::Header => "header".to_string(),
                openapiv3::APIKeyLocation::Cookie => "cookie".to_string(),
            }),
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
            extensions: extensions.clone(),
        },
        openapiv3::SecurityScheme::HTTP {
            scheme,
            bearer_format,
            description,
            extensions,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::Http,
            description: description.clone(),
            name: None,
            location: None,
            scheme: Some(scheme.clone()),
            bearer_format: bearer_format.clone(),
            flows: None,
            open_id_connect_url: None,
            extensions: extensions.clone(),
        },
        openapiv3::SecurityScheme::OAuth2 {
            flows,
            description,
            extensions,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::OAuth2,
            description: description.clone(),
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: Some(convert_oauth_flows(flows)),
            open_id_connect_url: None,
            extensions: extensions.clone(),
        },
        openapiv3::SecurityScheme::OpenIDConnect {
            open_id_connect_url,
            description,
            extensions,
        } => SecurityScheme {
            scheme_type: SecuritySchemeType::OpenIdConnect,
            description: description.clone(),
            name: None,
            location: None,
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: Some(open_id_connect_url.clone()),
            extensions: extensions.clone(),
        },
    }
}

fn convert_oauth_flows(flows: &openapiv3::OAuth2Flows) -> OAuthFlows {
    OAuthFlows {
        implicit: flows.implicit.as_ref().map(|f| OAuthFlow {
            authorization_url: Some(f.authorization_url.clone()),
            token_url: None,
            refresh_url: f.refresh_url.clone(),
            scopes: f.scopes.clone(),
        }),
        password: flows.password.as_ref().map(|f| OAuthFlow {
            authorization_url: None,
            token_url: Some(f.token_url.clone()),
            refresh_url: f.refresh_url.clone(),
            scopes: f.scopes.clone(),
        }),
        client_credentials: flows.client_credentials.as_ref().map(|f| OAuthFlow {
            authorization_url: None,
            token_url: Some(f.token_url.clone()),
            refresh_url: f.refresh_url.clone(),
            scopes: f.scopes.clone(),
        }),
        authorization_code: flows.authorization_code.as_ref().map(|f| OAuthFlow {
            authorization_url: Some(f.authorization_url.clone()),
            token_url: Some(f.token_url.clone()),
            refresh_url: f.refresh_url.clone(),
            scopes: f.scopes.clone(),
        }),
    }
}

fn convert_security_requirement(req: &IndexMap<String, Vec<String>>) -> SecurityRequirement {
    req.clone()
}

fn convert_tag(tag: &openapiv3::Tag) -> Tag {
    Tag {
        name: tag.name.clone(),
        description: tag.description.clone(),
    }
}
