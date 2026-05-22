//! Concrete error types for the savefile parser.
//! Callers switch on variants; no `anyhow` here per design spec §11.1.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected EOF: wanted {wanted} bytes, only {available} available at offset {at}")]
    UnexpectedEof {
        wanted: usize,
        available: usize,
        at: usize,
    },

    // NOTE: the bounds in this error message are duplicated from versions.rs
    // (MIN_SUPPORTED_HEADER_TYPE..=MAX_KNOWN_HEADER_TYPE). Keep in sync.
    #[error("unsupported save_header_type {found} (only 7..=14 are recognized by this build)")]
    UnsupportedHeaderType { found: i32 },

    #[error("unsupported save_version {found} (only >= 41 is supported by this build)")]
    UnsupportedSaveVersion { found: i32 },

    #[error("invalid UTF-8 in string at offset {at}: {source}")]
    InvalidUtf8 {
        at: usize,
        #[source]
        source: std::str::Utf8Error,
    },

    #[error("invalid UTF-16 in string at offset {at}")]
    InvalidUtf16 { at: usize },

    #[error("zlib decompression failed at offset {at}: {source}")]
    ZlibInflate {
        at: usize,
        #[source]
        source: miniz_oxide::inflate::DecompressError,
    },
}

#[allow(unused_attributes)] // #[must_use] on type alias is not yet enforced by this rustc version but documents intent
#[must_use]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_eof_message_is_actionable() {
        let e = Error::UnexpectedEof {
            wanted: 4,
            available: 1,
            at: 42,
        };
        let s = e.to_string();
        assert!(s.contains('4'), "should mention bytes wanted: {s}");
        assert!(s.contains('1'), "should mention bytes available: {s}");
        assert!(s.contains("42"), "should mention offset: {s}");
    }

    #[test]
    fn unsupported_header_type_message_includes_value() {
        let e = Error::UnsupportedHeaderType { found: 123 };
        assert!(
            e.to_string().contains("123"),
            "should include the bad value"
        );
    }

    #[test]
    fn zlib_inflate_message_includes_offset_and_source() {
        // Construct a deliberately-broken zlib stream and use the real DecompressError type.
        // The zlib header byte 0x00 is invalid, so this will reliably fail.
        let bad: &[u8] = &[0x00];
        let underlying = miniz_oxide::inflate::decompress_to_vec_zlib(bad).unwrap_err();
        let e = Error::ZlibInflate { at: 12345, source: underlying };
        let s = e.to_string();
        assert!(s.contains("12345"), "should mention offset: {s}");
        assert!(s.contains("zlib"), "should mention 'zlib': {s}");
    }
}
