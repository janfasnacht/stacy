//! Dependency analysis for Stata scripts
//!
//! Parses .do files to extract dependency information from:
//! - `do "file.do"` statements
//! - `run "file.do"` statements
//! - `include "file.do"` statements

pub mod parser;
pub mod tree;

// Re-export main types for library users
#[allow(unused_imports)]
pub use parser::{Dependency, DependencyType};
#[allow(unused_imports)]
pub use tree::DependencyTree;
