//! Error type for `scim-world`.

use thiserror::Error;

/// Errors emitted by `scim-world`.
#[derive(Debug, Error)]
pub enum Error {
    /// Underlying `scim-store` failure (DB read, decode).
    #[error("scim-store error: {0}")]
    Store(#[from] scim_store::Error),
}

/// Crate result alias.
#[must_use]
#[allow(unused_attributes)]
pub type Result<T> = std::result::Result<T, Error>;
