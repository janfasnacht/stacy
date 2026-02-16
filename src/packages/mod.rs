pub mod cache;
pub mod github;
pub mod global_cache;
pub mod installer;
pub mod lockfile;
pub mod pkg_parser;
pub mod ssc;

// Package types are defined in project/mod.rs
// Re-export them here for convenience (currently unused during development)
#[allow(unused_imports)]
pub use crate::project::{PackageEntry, PackageSource};

// Re-export commonly used types
#[allow(unused_imports)]
pub use installer::{install_package, install_package_github, InstallResult};
#[allow(unused_imports)]
pub use pkg_parser::PackageManifest;
#[allow(unused_imports)]
pub use ssc::{PackageDownload, SscDownloader};
