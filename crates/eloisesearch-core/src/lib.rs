//! Eloisesearch search core.
//!
//! This crate owns index lifecycle, schema, documents and query execution.
//! It is deliberately independent of HTTP or any other transport: the server
//! crate translates requests into calls on this API, and no Tantivy type is
//! part of the public surface.

mod data_dir;
mod error;
mod fsio;
mod index_name;
mod indexes;

pub use data_dir::{DataDir, FORMAT_VERSION, NodeMetadata};
pub use error::{Error, Result};
pub use index_name::IndexName;
pub use indexes::IndexMetadata;

/// Version of the Eloisesearch core, taken from the crate manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
