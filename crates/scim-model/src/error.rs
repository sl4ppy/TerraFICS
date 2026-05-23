//! Concrete error types for the typed-domain layer.
//! Callers switch on variants; no `anyhow` per design spec §11.1.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("savefile error: {0}")]
    Savefile(#[from] scim_savefile::Error),

    #[error("TOML parse error in {path:?}: {source}")]
    TomlParse {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("I/O error reading {path:?}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("no Component registered for class {class_name:?}")]
    NoComponentForClass { class_name: String },

    #[error("ConveyorBelt decode: expected count + items_length + items at the end of entity body, found {bytes_remaining} bytes remaining after decode")]
    ConveyorBeltTrailingBytes { bytes_remaining: usize },

    #[error("ConveyorChainActor decode: {bytes_remaining} bytes remaining after decode (expected 0)")]
    ConveyorChainActorTrailingBytes { bytes_remaining: usize },
}

#[allow(unused_attributes)]
#[must_use]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn savefile_error_converts_via_from() {
        let underlying = scim_savefile::Error::UnexpectedEof {
            wanted: 4,
            available: 1,
            at: 0,
        };
        let wrapped: Error = underlying.into();
        let s = wrapped.to_string();
        assert!(s.contains("savefile"));
    }
}
