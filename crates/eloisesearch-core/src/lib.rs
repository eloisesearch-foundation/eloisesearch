//! Eloisesearch search core.
//!
//! This crate owns index lifecycle, schema, documents and query execution.
//! It is deliberately independent of HTTP or any other transport: the server
//! crate translates requests into calls on this API, and no Tantivy type is
//! part of the public surface.

mod error;
mod index_name;

pub use error::{Error, Result};
pub use index_name::IndexName;

/// Version of the Eloisesearch core, taken from the crate manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
