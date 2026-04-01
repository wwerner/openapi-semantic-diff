# osd -- OpenAPI Semantic Diff

[![CI](https://github.com/wwerner/openapi-semantic-diff/actions/workflows/ci.yml/badge.svg)](https://github.com/wwerner/openapi-semantic-diff/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`osd` compares two OpenAPI specifications and produces a structured, severity-classified changeset. It supports OpenAPI 3.0.x and 3.1.x in both YAML and JSON, auto-detecting version and format.

Changes are classified into three severity tiers:

| Severity       | Meaning                                            |
|----------------|----------------------------------------------------|
| **Breaking**   | Consumers will break if they don't adapt            |
| **Deprecated** | Something newly marked deprecated; not broken yet   |
| **Additive**   | New capabilities added; no consumer impact          |

## Installation

### From source

Requires **Rust 1.70+** (2021 edition). Install via [rustup](https://rustup.rs/):

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then build and install:

```
cargo install --path .
```

### Pre-built binaries

Download pre-built binaries for your platform from the
[Releases](https://github.com/wwerner/openapi-semantic-diff/releases) page.

## Build

```
cargo build --release
```

The binary is written to `target/release/osd`.

To run the test suite:

```
cargo test
```

## Usage

### Basic comparison

```
osd diff old-spec.yaml new-spec.yaml
```

Prints a plain-text report to stdout. Exit codes are CI-friendly: `0` = no changes or success, `1` = breaking changes found (`check`), `2` = error.

Input files can be YAML or JSON in any combination -- format is detected automatically from file content.

### Output formats

Four built-in formats are available: `text` (default), `markdown`, `json`, `html`.

```
osd diff old.yaml new.yaml -f markdown
osd diff old.yaml new.yaml -f json
osd diff old.yaml new.yaml -f html
```

### CI gate

Exit `1` if breaking changes exist, `0` otherwise:

```
osd check old.yaml new.yaml
```

### Severity filtering

Only report changes at or above a given severity:

```
osd diff old.yaml new.yaml --min-severity breaking
osd diff old.yaml new.yaml --min-severity deprecated
```

### Writing output to a file

```
osd diff old.yaml new.yaml -f html -o report.html
```

### Using a custom template

```
osd diff old.yaml new.yaml -t my_template.tera
```

When `-t` is provided it overrides `-f`. See [Custom templates](#custom-templates) below.

### Exporting a built-in template

Dump a built-in template to disk so you can use it as a starting point:

```
osd templates text my_text.tera
osd templates markdown my_md.tera
osd templates json my_json.tera
osd templates html my_html.tera
```

### Full option reference

```
osd <COMMAND>

Commands:
  diff       Compare two OpenAPI specs and report changes
  check      Exit 1 if breaking changes are found, 0 otherwise (CI gate)
  templates  Export a built-in template to a file for customization
  help       Print this message or the help of the given subcommand(s)

osd diff [OPTIONS] <OLD> <NEW>

Arguments:
  <OLD>    Path to the old (base) OpenAPI spec
  <NEW>    Path to the new (changed) OpenAPI spec

Options:
  -f, --format <FORMAT>            Output format: text, markdown, json, html [default: text]
  -t, --template <FILE>            Custom Tera template file (overrides --format)
  -o, --output <FILE>              Write output to file instead of stdout
      --min-severity <SEVERITY>    Minimum severity: additive, deprecated, breaking [default: additive]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Extending

### Extension processors

OpenAPI `x-*` extensions are diffed through a pluggable processor system. By default, all extension changes (added, removed, modified) are classified as `Additive`. You can register custom processors that apply different severity rules for specific extension keys.

**Built-in processor: `x-extensible-enum`**

The `x-extensible-enum` extension is a common convention for enum values that are expected to grow over time. `osd` ships a dedicated processor for it:

- Values added to the enum -- `Additive`
- Values removed from the enum -- `Breaking`
- Entire extension added -- `Additive`
- Entire extension removed -- `Breaking`

**Writing a custom processor**

Implement the `ExtensionProcessor` trait from `openapi_semantic_diff::extension`:

```rust
use openapi_semantic_diff::extension::ExtensionProcessor;
use openapi_semantic_diff::model::{Change, ChangeType, Severity};

pub struct MyExtensionProcessor;

impl ExtensionProcessor for MyExtensionProcessor {
    /// The extension key this processor handles.
    fn key(&self) -> &str {
        "x-my-extension"
    }

    /// Produce changes given old and new values at the given path.
    fn process(
        &self,
        path: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Vec<Change> {
        let ext_path = format!("{path}.x-my-extension");
        match (old_value, new_value) {
            (None, Some(new)) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Added,
                severity: Severity::Additive,
                message: "x-my-extension added".into(),
                old_value: None,
                new_value: Some(new.clone()),
            }],
            (Some(old), None) => vec![Change {
                path: ext_path,
                change_type: ChangeType::Removed,
                severity: Severity::Breaking,
                message: "x-my-extension removed".into(),
                old_value: Some(old.clone()),
                new_value: None,
            }],
            (Some(old), Some(new)) if old != new => vec![Change {
                path: ext_path,
                change_type: ChangeType::Modified,
                severity: Severity::Breaking,
                message: "x-my-extension changed".into(),
                old_value: Some(old.clone()),
                new_value: Some(new.clone()),
            }],
            _ => vec![],
        }
    }
}
```

Register it on an `ExtensionRegistry` and pass that to the comparator:

```rust
use openapi_semantic_diff::extension::ExtensionRegistry;
use openapi_semantic_diff::comparator::compare_with_extensions;
use openapi_semantic_diff::parser;

let old = parser::parse_file("old.yaml").unwrap();
let new = parser::parse_file("new.yaml").unwrap();

let mut registry = ExtensionRegistry::with_defaults();
registry.register(Box::new(MyExtensionProcessor));

let report = compare_with_extensions(&old, &new, &registry);
```

`ExtensionRegistry::with_defaults()` includes the built-in `x-extensible-enum` processor. Use `ExtensionRegistry::new()` for an empty registry if you want full control.

### Custom templates

Output is rendered through [Tera](https://keats.github.io/tera/) templates (Jinja2-style syntax). The fastest way to get started is to export a built-in template and modify it:

```
osd templates text my_template.tera
# edit my_template.tera
osd diff old.yaml new.yaml -t my_template.tera
```

**Template context**

Every template receives the following variables:

| Variable                    | Type            | Description                                         |
|-----------------------------|-----------------|-----------------------------------------------------|
| `changes`                   | array of Change | All changes (flat list across all categories)       |
| `max_severity`              | string or null  | Highest severity found (`"breaking"`, `"deprecated"`, `"additive"`, or null) |
| `info_changes`              | array of Change | Changes to `info` (title, version, description)     |
| `server_changes`            | array of Change | Server additions/removals                           |
| `path_changes`              | array of Change | Endpoint and operation changes                      |
| `schema_changes`            | array of Change | Component schema changes                            |
| `security_scheme_changes`   | array of Change | Security scheme changes                             |
| `tag_changes`               | array of Change | Tag additions/removals                              |
| `extension_changes`         | array of Change | Top-level `x-*` extension changes                   |

Each **Change** object has these fields:

| Field         | Type           | Description                                                |
|---------------|----------------|------------------------------------------------------------|
| `path`        | string         | Dotted path to the changed element (e.g. `paths./pets.GET`) |
| `change_type` | string         | One of: `added`, `removed`, `modified`, `deprecated`       |
| `severity`    | string         | One of: `breaking`, `deprecated`, `additive`               |
| `message`     | string         | Human-readable description of the change                   |
| `old_value`   | JSON value or null | The previous value, if applicable                      |
| `new_value`   | JSON value or null | The new value, if applicable                           |

**Example: minimal custom template**

```
Total changes: {{ changes | length }}
{% for change in changes -%}
  [{{ change.severity }}] {{ change.message }}
{% endfor %}
```

**Tera filter reference**

Useful built-in Tera filters for templates:

- `{{ changes | length }}` -- array length
- `{{ changes | filter(attribute="severity", value="breaking") }}` -- filter by field value
- `{{ value | upper }}` -- uppercase a string
- `{{ value | default(value="n/a") }}` -- fallback for null values

Full Tera documentation: <https://keats.github.io/tera/docs/>

## Examples

See the [`examples/`](examples/) directory for sample input specs and outputs in all four formats.

<details>
<summary>Markdown output showcase (click to expand)</summary>

Markdown output (`-f markdown`) from a showcase diff covering every severity and change type across paths, parameters, and schemas:

---

## API Changes

**39** change(s) detected | Max severity: **breaking**

## Paths

### `/users`

#### GET

- 🔴➖ parameter 'fields' (query) removed — `parameters.fields.query`

- 🔴✏️ parameter 'limit' is now required — `parameters.limit.query`

- 🟡 parameter 'offset' marked as deprecated — `parameters.offset.query`

- 🟢➕ parameter 'sort' (query) added — `parameters.sort.query`

- 🟢➕ property 'avatar_url' added — `responses.200.content.application/json.schema.items.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.200.content.application/json.schema.items.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.items.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.items.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.items.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.items.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.200.content.application/json.schema.items.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.200.content.application/json.schema.items.properties.name`

- 🟡 schema marked as deprecated — `responses.200.content.application/json.schema.items.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.200.content.application/json.schema.items.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.200.content.application/json.schema.items.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.200.content.application/json.schema.items.properties.role.enum`

- 🔴➖ response '500' removed — `responses.500`

#### POST

- 🟢➕ property 'avatar_url' added — `responses.201.content.application/json.schema.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.201.content.application/json.schema.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.201.content.application/json.schema.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.201.content.application/json.schema.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.201.content.application/json.schema.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.201.content.application/json.schema.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.201.content.application/json.schema.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.201.content.application/json.schema.properties.name`

- 🟡 schema marked as deprecated — `responses.201.content.application/json.schema.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.201.content.application/json.schema.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.201.content.application/json.schema.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.201.content.application/json.schema.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.201.content.application/json.schema.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.201.content.application/json.schema.properties.role.enum`

### `/users/{userId}`

#### GET

- 🟢➕ property 'avatar_url' added — `responses.200.content.application/json.schema.properties.avatar_url`
- 🟢➕ [schema: User] property 'avatar_url' added — `responses.200.content.application/json.schema.properties.avatar_url`

- 🔴✏️ maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.properties.email.maxLength`
- 🔴✏️ [schema: User] maxLength reduced from 200 to 100 — `responses.200.content.application/json.schema.properties.email.maxLength`

- 🟢✏️ minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.properties.email.minLength`
- 🟢✏️ [schema: User] minLength reduced from 5 to 1 — `responses.200.content.application/json.schema.properties.email.minLength`

- 🔴➖ property 'name' removed — `responses.200.content.application/json.schema.properties.name`
- 🔴➖ [schema: User] property 'name' removed — `responses.200.content.application/json.schema.properties.name`

- 🟡 schema marked as deprecated — `responses.200.content.application/json.schema.properties.nickname`
- 🟡 [schema: User] schema marked as deprecated — `responses.200.content.application/json.schema.properties.nickname`

- 🟢➕ enum value "moderator" added — `responses.200.content.application/json.schema.properties.role.enum`
- 🟢➕ [schema: User] enum value "moderator" added — `responses.200.content.application/json.schema.properties.role.enum`
- 🔴➖ enum value "viewer" removed — `responses.200.content.application/json.schema.properties.role.enum`
- 🔴➖ [schema: User] enum value "viewer" removed — `responses.200.content.application/json.schema.properties.role.enum`

#### DELETE

- 🟡 operation marked as deprecated

### `/users/{userId}/avatar`

#### PUT

- 🔴➖ endpoint PUT /users/{userId}/avatar removed

### `/users/{userId}/settings`

#### GET

- 🟢➕ endpoint GET /users/{userId}/settings added

## Metadata

### Info

- 🟢✏️ version changed from '1.0.0' to '2.0.0' — `version`

### Schemas > LegacyProfile

- 🔴➖ schema 'LegacyProfile' removed — `components.schemas.LegacyProfile`

### Schemas > Settings

- 🟢➕ schema 'Settings' added — `components.schemas.Settings`

---

</details>

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)
