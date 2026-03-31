use izel_pm::{build_download_url, parse_manifest, resolve_dependencies_with_registry, Dependency};
use std::collections::HashMap;

#[test]
fn parse_manifest_reads_registry_section() {
    let input = r#"
[package]
name = "demo"
version = "0.1.0"

[registry]
index = "https://registry.example/index"
api = "https://registry.example/api/v1"
download = "https://registry.example/crates"

[dependencies]
std = "1.0.0"
"#;

    let manifest = parse_manifest(input).expect("manifest parse should succeed");
    assert_eq!(manifest.registry.index, "https://registry.example/index");
    assert_eq!(manifest.registry.api, "https://registry.example/api/v1");
    assert_eq!(
        manifest.registry.download,
        "https://registry.example/crates"
    );
}

#[test]
fn build_download_url_uses_public_registry_shape() {
    let url = build_download_url("https://registry.izel.dev/crates", "std", "1.0.0");
    assert_eq!(url, "https://registry.izel.dev/crates/std/1.0.0");
}

#[test]
fn resolve_dependencies_collects_registry_downloads() {
    let manifest = parse_manifest(include_str!("../../../Izel.toml"))
        .expect("workspace manifest should parse");

    let resolved = resolve_dependencies_with_registry(&manifest.dependencies, &manifest.registry)
        .expect("resolution should succeed");

    assert!(
        resolved.iter().any(|url| url.contains("/std/1.0.0")),
        "expected std version dependency to resolve from registry"
    );
}

#[test]
fn resolve_dependencies_skips_path_deps_for_downloads() {
    let registry = parse_manifest(include_str!("../../../Izel.toml"))
        .expect("workspace manifest should parse")
        .registry;

    let mut deps = HashMap::new();
    deps.insert("std".to_string(), Dependency::Version("1.0.0".to_string()));
    deps.insert(
        "core".to_string(),
        Dependency::Path("{ path = \"../core\" }".to_string()),
    );

    let resolved =
        resolve_dependencies_with_registry(&deps, &registry).expect("resolution should succeed");

    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].contains("/std/1.0.0"));
}

#[test]
fn parse_manifest_reports_parse_error_for_invalid_input() {
    let invalid = "[package\nname = \"demo\"";
    let err = parse_manifest(invalid).expect_err("invalid manifest should fail");
    assert!(err.contains("Failed to parse manifest"));
}

#[test]
fn parse_manifest_ignores_unknown_sections_and_preserves_defaults() {
    let input = r#"
[package]
name = "demo"
version = "0.1.0"

[custom]
feature = "on"
"#;

    let manifest = parse_manifest(input).expect("manifest parse should succeed");
    assert_eq!(manifest.package.name, "demo");
    assert_eq!(manifest.package.version, "0.1.0");
    assert_eq!(manifest.registry.index, "https://registry.izel.dev/index");
}

#[test]
fn parse_manifest_accepts_whitespace_only_input() {
    let manifest = parse_manifest("  \n\n   ").expect("whitespace input should parse");
    assert!(manifest.package.name.is_empty());
    assert!(manifest.package.version.is_empty());
    assert!(manifest.dependencies.is_empty());
}
