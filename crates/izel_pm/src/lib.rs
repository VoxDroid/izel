pub mod manifest;
pub mod resolve;

pub use manifest::{parse_manifest, Dependency, Manifest, PackageInfo, RegistryConfig};
pub use resolve::{
    build_download_url, fetch_package, resolve_dependencies, resolve_dependencies_with_registry,
};
