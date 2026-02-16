// Library interface for testing

#![allow(dead_code)] // Allow unused code during early development
#![allow(clippy::enum_variant_names)] // Error types have Error suffix intentionally
#![allow(clippy::upper_case_acronyms)] // SSC, JSON, etc. are standard acronyms

pub mod cache;
pub mod cli;
pub mod deps;
pub mod error;
pub mod executor;
pub mod metrics;
pub mod packages;
pub mod project;
pub mod task;
pub mod test;
pub mod update_check;
pub mod utils;
