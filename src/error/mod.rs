pub mod categories;
pub mod codes;
pub mod error_db;
pub mod extraction;
pub mod mapper;
pub mod parser;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Stata execution failed: {0}")]
    Execution(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Project not found. Run `stacy init` to create a project.")]
    ProjectNotFound,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    SyntaxError,      // r(198), r(199) — exit code 2
    FileError,        // r(601), r(603) — exit code 3
    MemoryError,      // r(950)         — exit code 4
    SystemError,      // r(800s)        — exit code 10
    StatisticalError, // r(400s)        — exit code 6
    StataError,       // Generic        — exit code 1
}

#[derive(Debug, Clone)]
pub enum StataError {
    /// Stata returned an r() error code
    StataCode {
        error_type: ErrorType,
        message: String,
        line_number: Option<usize>,
        r_code: u32,
    },
    /// Process was killed (SIGTERM, SIGINT, SIGKILL)
    ProcessKilled { exit_code: i32 },
}

impl StataError {
    pub fn new(error_type: ErrorType, message: String, r_code: u32) -> Self {
        Self::StataCode {
            error_type,
            message,
            line_number: None,
            r_code,
        }
    }

    pub fn with_line_number(self, line_number: usize) -> Self {
        match self {
            Self::StataCode {
                error_type,
                message,
                r_code,
                ..
            } => Self::StataCode {
                error_type,
                message,
                line_number: Some(line_number),
                r_code,
            },
            other => other,
        }
    }

    pub fn r_code(&self) -> Option<u32> {
        match self {
            Self::StataCode { r_code, .. } => Some(*r_code),
            Self::ProcessKilled { .. } => None,
        }
    }

    pub fn error_type(&self) -> ErrorType {
        match self {
            Self::StataCode { error_type, .. } => *error_type,
            Self::ProcessKilled { .. } => ErrorType::StataError,
        }
    }
}
