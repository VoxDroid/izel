pub mod manifest;
pub mod resolve;

pub use manifest::{parse_manifest, Dependency, Manifest, PackageInfo};
pub use resolve::{fetch_package, resolve_dependencies};
