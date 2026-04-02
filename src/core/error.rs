use std::path::PathBuf;

use thiserror::Error;

/// Crate-wide result type.
pub type Result<T> = std::result::Result<T, ForgeError>;

/// Errors produced by the conversion pipeline.
#[derive(Debug, Error)]
pub enum ForgeError {
    /// No markdown source was configured before conversion.
    #[error("a markdown source must be provided before conversion")]
    MissingSource,
    /// No output target was configured before conversion.
    #[error("an output target must be provided before conversion")]
    MissingOutput,
    /// Reading a path-based markdown source failed.
    #[error("failed to read markdown input from {path}: {source}")]
    InputRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Byte input was not valid UTF-8.
    #[error("input bytes are not valid UTF-8")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    /// Markdown parsing failed.
    #[error("failed to parse markdown: {message}")]
    Parse { message: String },
    /// A resource resolver returned a hard failure.
    #[error("resource resolution failed for {target}: {message}")]
    Resource { target: String, message: String },
    /// A supplied runtime configuration value was invalid.
    #[error("invalid configuration for {field}: {message}")]
    InvalidConfiguration {
        field: &'static str,
        message: String,
    },
    /// The selected renderer failed.
    #[error("renderer failed: {message}")]
    Render { message: String },
    /// Typst compilation failed.
    #[error("typst compilation failed: {message}")]
    TypstCompile { message: String },
    /// Typst PDF export failed.
    #[error("typst PDF export failed: {message}")]
    TypstExport { message: String },
    /// Typst-specific asset preparation failed.
    #[error("typst asset {target} is unavailable: {message}")]
    TypstAsset { target: String, message: String },
    /// Writing the final PDF to a file failed.
    #[error("failed to write PDF to {path}: {source}")]
    OutputWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Creating the output directory failed.
    #[error("failed to create output directory {path}: {source}")]
    OutputDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A caller-supplied directory output file name was empty after trimming.
    #[error("the output file name cannot be empty")]
    InvalidOutputFileName,
}
