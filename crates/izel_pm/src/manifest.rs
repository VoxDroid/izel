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
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
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
            _ => {}
        }
    }

    Ok(Manifest {
        package,
        dependencies,
    })
}
