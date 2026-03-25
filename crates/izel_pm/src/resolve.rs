use crate::manifest::{Dependency, RegistryConfig};

pub fn resolve_dependencies(
    deps: &std::collections::HashMap<String, Dependency>,
) -> Result<(), String> {
    resolve_dependencies_with_registry(deps, &RegistryConfig::default()).map(|_| ())
}

pub fn resolve_dependencies_with_registry(
    deps: &std::collections::HashMap<String, Dependency>,
    registry: &RegistryConfig,
) -> Result<Vec<String>, String> {
    let mut resolved_downloads = Vec::new();
    for (name, dep) in deps {
        match dep {
            Dependency::Version(v) => {
                println!(
                    "Resolving {} v{} via public registry index {}...",
                    name, v, registry.index
                );
                let url = fetch_package(name, v, registry)?;
                resolved_downloads.push(url);
            }
            Dependency::Path(p) => {
                println!("Resolving {} via local path {}...", name, p);
            }
        }
    }
    Ok(resolved_downloads)
}

pub fn fetch_package(
    name: &str,
    version: &str,
    registry: &RegistryConfig,
) -> Result<String, String> {
    let download_url = build_download_url(&registry.download, name, version);
    println!(
        "Fetching {}@{} from public registry via {}",
        name, version, download_url
    );
    Ok(download_url)
}

pub fn build_download_url(download_base: &str, name: &str, version: &str) -> String {
    let base = download_base.trim_end_matches('/');
    format!("{}/{}/{}", base, name, version)
}

#[cfg(test)]
mod tests {
    use super::{build_download_url, resolve_dependencies_with_registry};
    use crate::manifest::{Dependency, RegistryConfig};
    use std::collections::HashMap;

    #[test]
    fn build_download_url_normalizes_trailing_slash() {
        let url = build_download_url("https://registry.izel.dev/crates/", "std", "1.0.0");
        assert_eq!(url, "https://registry.izel.dev/crates/std/1.0.0");
    }

    #[test]
    fn resolve_dependencies_uses_public_registry_for_versions() {
        let mut deps = HashMap::new();
        deps.insert("std".to_string(), Dependency::Version("1.0.0".to_string()));
        deps.insert(
            "core".to_string(),
            Dependency::Path("{ path = \"../core\" }".to_string()),
        );

        let registry = RegistryConfig {
            index: "https://registry.izel.dev/index".to_string(),
            api: "https://registry.izel.dev/api/v1".to_string(),
            download: "https://registry.izel.dev/crates".to_string(),
        };

        let resolved = resolve_dependencies_with_registry(&deps, &registry)
            .expect("resolution should succeed");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], "https://registry.izel.dev/crates/std/1.0.0");
    }
}
