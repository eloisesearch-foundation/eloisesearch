use thiserror::Error;

/// Result alias used across the core.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by the search core.
///
/// Variants describe *what* went wrong in domain terms; mapping them to
/// transport-level status codes is the server's responsibility.
#[derive(Debug, Error, PartialEq, Eq)]
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
}
