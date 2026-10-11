use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Result alias used across the core.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by the search core.
///
/// Variants describe *what* went wrong in domain terms; mapping them to
/// transport-level status codes is the server's responsibility.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An index name does not satisfy the naming rules.
    #[error("invalid index name {name:?}: {reason}")]
    InvalidIndexName {
        /// The rejected name.
        name: String,
        /// Human-readable explanation of the violated rule.
        reason: &'static str,
    },

    /// Another process already uses this data directory.
    #[error("data directory {path} is already in use by another Eloisesearch process")]
    DataDirLocked {
        /// The data directory.
        path: PathBuf,
    },

    /// A metadata file was written by a newer, incompatible version.
    #[error(
        "{path} has format version {found}, this build supports up to {supported}; upgrade Eloisesearch"
    )]
    UnsupportedFormat {
        /// The metadata file.
        path: PathBuf,
        /// Format version found in the file.
        found: u32,
        /// Highest format version this build understands.
        supported: u32,
    },

    /// A metadata file exists but cannot be parsed.
    #[error("{path} is corrupt: {reason}")]
    CorruptMetadata {
        /// The metadata file.
        path: PathBuf,
        /// Why it could not be read.
        reason: String,
    },

    /// An index with this name already exists.
    #[error("index [{name}] already exists")]
    IndexAlreadyExists {
        /// The index name.
        name: String,
    },

    /// No index with this name exists.
    #[error("no such index [{name}]")]
    IndexNotFound {
        /// The index name.
        name: String,
    },

    /// A request argument is invalid (for example settings that are not an object).
    #[error("{0}")]
    InvalidArgument(String),

    /// An I/O operation on the data directory failed.
    #[error("{context}: {source}")]
    Io {
        /// What was being done, including the path.
        context: String,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
}

impl Error {
    /// Wraps an I/O error with a description of the operation.
    pub(crate) fn io(context: impl Into<String>) -> impl FnOnce(io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }
}
