use std::collections::HashMap;
use winnow::{
    ascii::space0,
    combinator::{delimited, separated_pair},
    token::take_till,
    ModalResult, Parser,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, Dependency>,
    pub registry: RegistryConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegistryConfig {
    pub index: String,
    pub api: String,
    pub download: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            index: "https://registry.izel.dev/index".to_string(),
            api: "https://registry.izel.dev/api/v1".to_string(),
            download: "https://registry.izel.dev/crates".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dependency {
    Version(String),
    Path(String),
}

pub fn parse_manifest(input: &str) -> Result<Manifest, String> {
    match manifest.parse(input) {
        Ok(m) => Ok(m),
        Err(e) => Err(format!("Failed to parse manifest: {:?}", e)),
    }
}

// A highly simplified TOML parser using winnow, sufficient for Phase 5.3 scaffolding
fn manifest(input: &mut &str) -> ModalResult<Manifest> {
    let mut package = PackageInfo {
        name: String::new(),
        version: String::new(),
    };
    let mut dependencies = HashMap::new();
    let mut registry = RegistryConfig::default();
    let mut current_section = "";

    // Parse line by line
    while !input.is_empty() {
        // Skip leading whitespace/newlines
        let _ = winnow::ascii::multispace0::<_, winnow::error::ContextError>.parse_next(input)?;
        if input.is_empty() {
            break;
        }

        // Is it a section header?
        if input.starts_with('[') {
            let section: &str = delimited('[', take_till(1.., ']'), ']').parse_next(input)?;
            current_section = section;
            let _ =
                winnow::ascii::multispace0::<_, winnow::error::ContextError>.parse_next(input)?;
            continue;
        }

        // Key-value pair
        let key_value: (&str, &str) = separated_pair(
            take_till(1.., |c: char| c == '=' || c.is_whitespace()),
            delimited(space0, '=', space0),
            take_till(1.., |c: char| c == '\n' || c == '\r'),
        )
        .parse_next(input)?;

        let key = key_value.0.trim();
        let value = key_value.1.trim().trim_matches('"');

        match current_section {
            "package" => {
                if key == "name" {
                    package.name = value.to_string();
                }
                if key == "version" {
                    package.version = value.to_string();
                }
            }
            "dependencies" => {
                // If it starts with {, it's likely a path or complex. Simpler fallback for now.
                if value.contains("path") {
                    dependencies.insert(key.to_string(), Dependency::Path(value.to_string()));
                } else {
                    dependencies.insert(key.to_string(), Dependency::Version(value.to_string()));
                }
            }
            "registry" => {
                if key == "index" {
                    registry.index = value.to_string();
                }
                if key == "api" {
                    registry.api = value.to_string();
                }
                if key == "download" {
                    registry.download = value.to_string();
                }
            }
            _ => {}
        }
    }

    Ok(Manifest {
        package,
        dependencies,
        registry,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_manifest, Dependency};

    #[test]
    fn parse_manifest_uses_default_registry_when_section_absent() {
        let input = r#"
[package]
name = "demo"
version = "0.1.0"

[dependencies]
std = "1.0.0"
"#;

        let manifest = parse_manifest(input).expect("manifest should parse");
        assert_eq!(manifest.registry.index, "https://registry.izel.dev/index");
        assert_eq!(manifest.registry.api, "https://registry.izel.dev/api/v1");
        assert_eq!(
            manifest.registry.download,
            "https://registry.izel.dev/crates"
        );
        assert_eq!(
            manifest.dependencies.get("std"),
            Some(&Dependency::Version("1.0.0".to_string()))
        );
    }

    #[test]
    fn parse_manifest_accepts_registry_overrides() {
        let input = r#"
[package]
name = "demo"
version = "0.1.0"

[registry]
index = "https://example.registry/index"
api = "https://example.registry/api/v1"
download = "https://example.registry/crates"

[dependencies]
std = "1.0.0"
"#;

        let manifest = parse_manifest(input).expect("manifest should parse");
        assert_eq!(manifest.registry.index, "https://example.registry/index");
        assert_eq!(manifest.registry.api, "https://example.registry/api/v1");
        assert_eq!(
            manifest.registry.download,
            "https://example.registry/crates"
        );
    }
}
