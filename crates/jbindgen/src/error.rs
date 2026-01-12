//! Error types for jbindgen

use std::io;

/// Result type alias for jbindgen operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during binding generation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error reading class file
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Error parsing class file with cafebabe
    #[error("Failed to parse class file: {0}")]
    ParseClass(String),

    /// Error parsing Java sources
    #[error("Failed to parse Java sources: {0}")]
    Parse(String),

    /// Error reading JAR file
    #[error("Failed to read JAR file: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Android SDK related error
    #[error("Android SDK error: {0}")]
    AndroidSdk(String),

    /// Unsupported class feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// Code generation error
    #[error("Code generation error: {0}")]
    CodeGen(String),
}

impl From<cafebabe::ParseError> for Error {
    fn from(err: cafebabe::ParseError) -> Self {
        Error::ParseClass(err.to_string())
    }
}
