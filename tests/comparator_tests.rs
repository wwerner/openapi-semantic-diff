// tests/comparator_tests.rs — Integration tests for the comparator

use openapi_semantic_diff::comparator::compare;
use openapi_semantic_diff::model::*;
use openapi_semantic_diff::parser;
use std::path::Path;

fn load(name: &str) -> InternalSpec {
    let path = Path::new("tests/fixtures").join(name);
    parser::parse_file(&path).unwrap_or_else(|e| panic!("failed to parse {name}: {e}"))
}

// ---------------------------------------------------------------------------
// No changes
// ---------------------------------------------------------------------------

#[test]
fn identical_specs_produce_empty_report() {
    let spec = load("empty/spec.yaml");
    let report = compare(&spec, &spec);
    assert!(report.is_empty(), "expected no changes, got: {:#?}", report);
}

// ---------------------------------------------------------------------------
// Breaking changes
// ---------------------------------------------------------------------------

#[test]
fn breaking_changes_detected() {
    // Any breaking fixture will do — use the one with the most variety.
    let old = load("removed_endpoint/base.yaml");
    let new = load("removed_endpoint/changed.yaml");
    let report = compare(&old, &new);

    assert!(!report.is_empty());
    assert_eq!(report.max_severity(), Some(Severity::Breaking));
}

#[test]
fn removed_endpoint_is_breaking() {
    let old = load("removed_endpoint/base.yaml");
    let new = load("removed_endpoint/changed.yaml");
    let report = compare(&old, &new);

    let removed = report.path_changes.iter().find(|c| {
        c.path.contains("/pets/{petId}")
            && c.path.contains("DELETE")
            && c.change_type == ChangeType::Removed
    });

    assert!(
        removed.is_some(),
        "expected DELETE /pets/{{petId}} to be detected as removed"
    );
    assert_eq!(removed.unwrap().severity, Severity::Breaking);
}

#[test]
fn removed_property_is_breaking() {
    let old = load("removed_property/base.yaml");
    let new = load("removed_property/changed.yaml");
    let report = compare(&old, &new);

    let removed = report.schema_changes.iter().find(|c| {
        c.path.contains("Pet")
            && c.path.contains("properties.name")
            && c.change_type == ChangeType::Removed
    });

    assert!(
        removed.is_some(),
        "expected Pet.name removal to be detected"
    );
    assert_eq!(removed.unwrap().severity, Severity::Breaking);
}

#[test]
fn type_change_is_breaking() {
    let old = load("type_change/base.yaml");
    let new = load("type_change/changed.yaml");
    let report = compare(&old, &new);

    let type_change = report.schema_changes.iter().find(|c| {
        c.path.contains("Pet")
            && c.path.contains("properties.id")
            && c.path.contains("type")
            && c.change_type == ChangeType::Modified
    });

    assert!(
        type_change.is_some(),
        "expected Pet.id type change to be detected"
    );
    assert_eq!(type_change.unwrap().severity, Severity::Breaking);
}

#[test]
fn parameter_made_required_is_breaking() {
    let old = load("required_parameter/base.yaml");
    let new = load("required_parameter/changed.yaml");
    let report = compare(&old, &new);

    let required_change = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("limit") && c.message.contains("required"));

    assert!(
        required_change.is_some(),
        "expected limit parameter required change to be detected"
    );
    assert_eq!(required_change.unwrap().severity, Severity::Breaking);
}

#[test]
fn removed_enum_value_is_breaking() {
    let old = load("removed_enum_value/base.yaml");
    let new = load("removed_enum_value/changed.yaml");
    let report = compare(&old, &new);

    let enum_removed = report.schema_changes.iter().find(|c| {
        c.path.contains("Pet")
            && c.path.contains("status")
            && c.path.contains("enum")
            && c.change_type == ChangeType::Removed
            && c.message.contains("sold")
    });

    assert!(
        enum_removed.is_some(),
        "expected 'sold' enum value removal to be detected"
    );
    assert_eq!(enum_removed.unwrap().severity, Severity::Breaking);
}

#[test]
fn removed_server_is_breaking() {
    let old = load("removed_server/base.yaml");
    let new = load("removed_server/changed.yaml");
    let report = compare(&old, &new);

    let server_removed = report
        .server_changes
        .iter()
        .find(|c| c.change_type == ChangeType::Removed);

    assert!(
        server_removed.is_some(),
        "expected server removal to be detected"
    );
    assert_eq!(server_removed.unwrap().severity, Severity::Breaking);
}

#[test]
fn removed_security_scheme_is_breaking() {
    let old = load("removed_security_scheme/base.yaml");
    let new = load("removed_security_scheme/changed.yaml");
    let report = compare(&old, &new);

    let scheme_removed = report
        .security_scheme_changes
        .iter()
        .find(|c| c.message.contains("apiKey") && c.change_type == ChangeType::Removed);

    assert!(
        scheme_removed.is_some(),
        "expected apiKey security scheme removal to be detected"
    );
    assert_eq!(scheme_removed.unwrap().severity, Severity::Breaking);
}

#[test]
fn reduced_max_length_is_breaking() {
    let old = load("reduced_max_length/base.yaml");
    let new = load("reduced_max_length/changed.yaml");
    let report = compare(&old, &new);

    let constraint = report.schema_changes.iter().find(|c| {
        c.path.contains("NewPet") && c.path.contains("maxLength") && c.message.contains("reduced")
    });

    assert!(
        constraint.is_some(),
        "expected NewPet.name maxLength reduction to be detected"
    );
    assert_eq!(constraint.unwrap().severity, Severity::Breaking);
}

// ---------------------------------------------------------------------------
// Additive changes
// ---------------------------------------------------------------------------

#[test]
fn additive_changes_detected() {
    let old = load("new_endpoint/base.yaml");
    let new = load("new_endpoint/changed.yaml");
    let report = compare(&old, &new);

    assert!(!report.is_empty());

    let breaking: Vec<_> = report
        .all_changes()
        .into_iter()
        .filter(|c| c.severity == Severity::Breaking)
        .collect();
    assert!(
        breaking.is_empty(),
        "expected no breaking changes, got: {:#?}",
        breaking
    );
}

#[test]
fn new_endpoint_is_additive() {
    let old = load("new_endpoint/base.yaml");
    let new = load("new_endpoint/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("/pets/{petId}/toys") && c.change_type == ChangeType::Added);

    assert!(
        added.is_some(),
        "expected GET /pets/{{petId}}/toys to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_optional_property_is_additive() {
    let old = load("new_property/base.yaml");
    let new = load("new_property/changed.yaml");
    let report = compare(&old, &new);

    let added = report.schema_changes.iter().find(|c| {
        c.path.contains("Pet") && c.path.contains("breed") && c.change_type == ChangeType::Added
    });

    assert!(
        added.is_some(),
        "expected Pet.breed addition to be detected"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_optional_parameter_is_additive() {
    let old = load("new_optional_parameter/base.yaml");
    let new = load("new_optional_parameter/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("sort") && c.change_type == ChangeType::Added);

    assert!(
        added.is_some(),
        "expected 'sort' parameter to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_response_code_is_additive() {
    let old = load("new_response_code/base.yaml");
    let new = load("new_response_code/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("429") && c.change_type == ChangeType::Added);

    assert!(
        added.is_some(),
        "expected 429 response to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_enum_value_is_additive() {
    let old = load("new_enum_value/base.yaml");
    let new = load("new_enum_value/changed.yaml");
    let report = compare(&old, &new);

    let added = report.schema_changes.iter().find(|c| {
        c.path.contains("status")
            && c.path.contains("enum")
            && c.change_type == ChangeType::Added
            && c.message.contains("adopted")
    });

    assert!(
        added.is_some(),
        "expected 'adopted' enum value to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_schema_is_additive() {
    let old = load("new_schema/base.yaml");
    let new = load("new_schema/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .schema_changes
        .iter()
        .find(|c| c.path.contains("Toy") && c.change_type == ChangeType::Added);

    assert!(
        added.is_some(),
        "expected Toy schema to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_server_is_additive() {
    let old = load("new_server/base.yaml");
    let new = load("new_server/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .server_changes
        .iter()
        .find(|c| c.change_type == ChangeType::Added);

    assert!(added.is_some(), "expected new server to be detected");
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

#[test]
fn new_tag_is_additive() {
    let old = load("new_tag/base.yaml");
    let new = load("new_tag/changed.yaml");
    let report = compare(&old, &new);

    let added = report
        .tag_changes
        .iter()
        .find(|c| c.message.contains("toys") && c.change_type == ChangeType::Added);

    assert!(
        added.is_some(),
        "expected 'toys' tag to be detected as added"
    );
    assert_eq!(added.unwrap().severity, Severity::Additive);
}

// ---------------------------------------------------------------------------
// Deprecation changes
// ---------------------------------------------------------------------------

#[test]
fn deprecation_changes_detected() {
    let old = load("deprecated_operation/base.yaml");
    let new = load("deprecated_operation/changed.yaml");
    let report = compare(&old, &new);

    assert!(!report.is_empty());

    let deprecated: Vec<_> = report
        .all_changes()
        .into_iter()
        .filter(|c| c.severity == Severity::Deprecated)
        .collect();

    assert!(
        !deprecated.is_empty(),
        "expected deprecation changes, got none"
    );
}

#[test]
fn deprecated_operation_detected() {
    let old = load("deprecated_operation/base.yaml");
    let new = load("deprecated_operation/changed.yaml");
    let report = compare(&old, &new);

    let dep = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("DELETE") && c.change_type == ChangeType::Deprecated);

    assert!(
        dep.is_some(),
        "expected DELETE operation deprecation to be detected"
    );
    assert_eq!(dep.unwrap().severity, Severity::Deprecated);
}

#[test]
fn deprecated_parameter_detected() {
    let old = load("deprecated_parameter/base.yaml");
    let new = load("deprecated_parameter/changed.yaml");
    let report = compare(&old, &new);

    let dep = report
        .path_changes
        .iter()
        .find(|c| c.path.contains("offset") && c.change_type == ChangeType::Deprecated);

    assert!(
        dep.is_some(),
        "expected 'offset' parameter deprecation to be detected"
    );
    assert_eq!(dep.unwrap().severity, Severity::Deprecated);
}

#[test]
fn deprecated_schema_property_detected() {
    let old = load("deprecated_schema_property/base.yaml");
    let new = load("deprecated_schema_property/changed.yaml");
    let report = compare(&old, &new);

    let dep = report.schema_changes.iter().find(|c| {
        c.path.contains("Pet") && c.path.contains("tag") && c.change_type == ChangeType::Deprecated
    });

    assert!(dep.is_some(), "expected Pet.tag deprecation to be detected");
    assert_eq!(dep.unwrap().severity, Severity::Deprecated);
}

// ---------------------------------------------------------------------------
// Severity filtering
// ---------------------------------------------------------------------------

#[test]
fn filter_by_severity() {
    let old = load("filter_severity/base.yaml");
    let new = load("filter_severity/changed.yaml");
    let report = compare(&old, &new);

    let filtered = report.filtered(Severity::Breaking);
    for change in filtered.all_changes() {
        assert_eq!(
            change.severity,
            Severity::Breaking,
            "filtered report should only contain breaking changes, got: {:#?}",
            change
        );
    }
}

// ---------------------------------------------------------------------------
// x-extensible-enum
// ---------------------------------------------------------------------------

#[test]
fn extensible_enum_removal_is_breaking() {
    let old = load("extensible_enum/base.yaml");
    let new = load("extensible_enum/changed_removal.yaml");
    let report = compare(&old, &new);

    let removed = report.schema_changes.iter().find(|c| {
        c.path.contains("x-extensible-enum")
            && c.severity == Severity::Breaking
            && c.message.contains("sold")
    });

    assert!(
        removed.is_some(),
        "expected x-extensible-enum 'sold' removal to be breaking"
    );
}

#[test]
fn extensible_enum_addition_is_additive() {
    let old = load("extensible_enum/base.yaml");
    let new = load("extensible_enum/changed_addition.yaml");
    let report = compare(&old, &new);

    let added = report.schema_changes.iter().find(|c| {
        c.path.contains("x-extensible-enum")
            && c.severity == Severity::Additive
            && c.message.contains("adopted")
    });

    assert!(
        added.is_some(),
        "expected x-extensible-enum 'adopted' addition to be additive"
    );
}
