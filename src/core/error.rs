use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ForgeError>;

/// Errors produced by the conversion pipeline.
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("a markdown source must be provided before conversion")]
    MissingSource,
    #[error("an output target must be provided before conversion")]
    MissingOutput,
    #[error("failed to read markdown input from {path}: {source}")]
    InputRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("input bytes are not valid UTF-8")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("failed to parse markdown: {message}")]
    Parse { message: String },
    #[error("resource resolution failed for {target}: {message}")]
    Resource { target: String, message: String },
    #[error("invalid configuration for {field}: {message}")]
    InvalidConfiguration {
        field: &'static str,
        message: String,
    },
    #[error("renderer failed: {message}")]
    Render { message: String },
    #[error("typst compilation failed: {message}")]
    TypstCompile { message: String },
    #[error("typst PDF export failed: {message}")]
    TypstExport { message: String },
    #[error("typst asset {target} is unavailable: {message}")]
    TypstAsset { target: String, message: String },
    #[error("failed to write PDF to {path}: {source}")]
    OutputWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create output directory {path}: {source}")]
    OutputDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the output file name cannot be empty")]
    InvalidOutputFileName,
}
