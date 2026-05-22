//! Decompresses the body of a `.sav` file.
//!
//! The body is a sequence of zlib-compressed chunks, each preceded by a fixed-size
//! `ChunkHeader` (see `chunk_header.rs`). `read_body` walks all chunks, decompresses
//! each, and concatenates the result into a single `Vec<u8>`.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:103-167.

use crate::chunk_header::read_chunk_header;
use crate::error::{Error, Result};
use crate::reader::Reader;

/// Decompress all body chunks. `body_bytes` must start at the first byte AFTER the
/// header — use the `consumed` value returned by `read_header` to obtain it.
pub fn read_body(body_bytes: &[u8], save_version: i32) -> Result<Vec<u8>> {
    let mut r = Reader::new(body_bytes);
    let mut out = Vec::new();

    while r.remaining() > 0 {
        let chunk_start = r.position();
        let h = read_chunk_header(&mut r, save_version)?;
        let compressed =
            r.read_hex(usize::try_from(h.compressed_size).expect("compressed_size fits usize"))?;

        let decompressed =
            miniz_oxide::inflate::decompress_to_vec_zlib(&compressed).map_err(|source| {
                Error::ZlibInflate {
                    at: chunk_start,
                    source,
                }
            })?;

        // Sanity: decompressed length should match the chunk's uncompressed_size.
        let actual = u64::try_from(decompressed.len()).expect("usize fits u64");
        if actual != h.uncompressed_size {
            return Err(Error::ChunkLengthMismatch {
                at: chunk_start,
                expected: h.uncompressed_size,
                actual,
            });
        }
        out.extend_from_slice(&decompressed);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_header::{CHUNK_HEADER_LEN_V41, COMPRESSION_FORMAT_ZLIB};
    use miniz_oxide::deflate::compress_to_vec_zlib;

    /// Build a single-chunk body from `payload`.
    fn one_chunk(payload: &[u8]) -> Vec<u8> {
        let compressed = compress_to_vec_zlib(payload, 6);
        let mut b = Vec::new();
        // 49-byte chunk header
        b.extend_from_slice(&0x9E2A_83C1_u64.to_le_bytes()); // package_file_tag
        b.extend_from_slice(&0x20000_u64.to_le_bytes()); // max_chunk_size
        b.push(COMPRESSION_FORMAT_ZLIB); // compression_format
        b.extend_from_slice(&(compressed.len() as u64).to_le_bytes()); // compressed
        b.extend_from_slice(&(payload.len() as u64).to_le_bytes()); // uncompressed
        b.extend_from_slice(&(compressed.len() as u64).to_le_bytes()); // dup
        b.extend_from_slice(&(payload.len() as u64).to_le_bytes()); // dup
        assert_eq!(b.len(), CHUNK_HEADER_LEN_V41);
        b.extend_from_slice(&compressed);
        b
    }

    #[test]
    fn read_body_one_chunk_roundtrip() {
        let payload = b"hello, world!".repeat(100);
        let body = one_chunk(&payload);
        let result = read_body(&body, 46).unwrap();
        assert_eq!(result, payload);
    }

    #[test]
    fn read_body_empty_input_returns_empty() {
        let result = read_body(&[], 46).unwrap();
        assert!(result.is_empty());
    }

    /// Build a body with N chunks, each containing `payload`.
    fn n_chunks(payload: &[u8], n: usize) -> Vec<u8> {
        let single = one_chunk(payload);
        let mut b = Vec::with_capacity(single.len() * n);
        for _ in 0..n {
            b.extend_from_slice(&single);
        }
        b
    }

    #[test]
    fn read_body_three_chunks_concatenates() {
        let payload = b"chunk content".repeat(50);
        let body = n_chunks(&payload, 3);
        let result = read_body(&body, 46).unwrap();
        // Result is payload * 3
        assert_eq!(result.len(), payload.len() * 3);
        assert_eq!(&result[..payload.len()], payload.as_slice());
        assert_eq!(
            &result[payload.len()..2 * payload.len()],
            payload.as_slice()
        );
        assert_eq!(&result[2 * payload.len()..], payload.as_slice());
    }

    #[test]
    fn read_body_corrupt_zlib_returns_zlib_inflate_error() {
        // Build a chunk where the "compressed" bytes are not actually valid zlib.
        let bogus = vec![0xFF_u8; 32];
        let mut b = Vec::new();
        b.extend_from_slice(&0x9E2A_83C1_u64.to_le_bytes());
        b.extend_from_slice(&0x20000_u64.to_le_bytes());
        b.push(COMPRESSION_FORMAT_ZLIB);
        b.extend_from_slice(&(u64::try_from(bogus.len()).unwrap()).to_le_bytes()); // compressed
        b.extend_from_slice(&100_u64.to_le_bytes()); // claimed uncompressed
        b.extend_from_slice(&(u64::try_from(bogus.len()).unwrap()).to_le_bytes());
        b.extend_from_slice(&100_u64.to_le_bytes());
        b.extend_from_slice(&bogus);
        let err = read_body(&b, 46).unwrap_err();
        assert!(matches!(err, Error::ZlibInflate { at: 0, .. }));
    }

    #[test]
    fn read_body_chunk_length_mismatch() {
        // Build a chunk where the chunk header CLAIMS 1000 uncompressed bytes
        // but the actual compressed payload only inflates to 13.
        let payload = b"hello, world!"; // 13 bytes
        let compressed = compress_to_vec_zlib(payload, 6);
        let mut b = Vec::new();
        b.extend_from_slice(&0x9E2A_83C1_u64.to_le_bytes());
        b.extend_from_slice(&0x20000_u64.to_le_bytes());
        b.push(COMPRESSION_FORMAT_ZLIB);
        b.extend_from_slice(&(u64::try_from(compressed.len()).unwrap()).to_le_bytes());
        b.extend_from_slice(&1000_u64.to_le_bytes()); // LIES: claims 1000 uncompressed
        b.extend_from_slice(&(u64::try_from(compressed.len()).unwrap()).to_le_bytes());
        b.extend_from_slice(&1000_u64.to_le_bytes());
        b.extend_from_slice(&compressed);
        let err = read_body(&b, 46).unwrap_err();
        assert!(matches!(
            err,
            Error::ChunkLengthMismatch {
                expected: 1000,
                actual: 13,
                ..
            }
        ));
    }

    #[test]
    fn read_body_truncated_chunk_payload() {
        // Build a chunk header that says compressed = 1000, but only provide 5 bytes.
        let mut b = Vec::new();
        b.extend_from_slice(&0x9E2A_83C1_u64.to_le_bytes());
        b.extend_from_slice(&0x20000_u64.to_le_bytes());
        b.push(COMPRESSION_FORMAT_ZLIB);
        b.extend_from_slice(&1000_u64.to_le_bytes());
        b.extend_from_slice(&500_u64.to_le_bytes());
        b.extend_from_slice(&1000_u64.to_le_bytes());
        b.extend_from_slice(&500_u64.to_le_bytes());
        b.extend_from_slice(&[0_u8; 5]); // way short of 1000
        let err = read_body(&b, 46).unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { wanted: 1000, .. }));
    }
}
