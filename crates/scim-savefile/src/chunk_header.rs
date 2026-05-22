//! Per-chunk header for the compressed body of a `.sav` file.
//!
//! For `save_version >= 41` (Update 8 and later), the chunk header is 49 bytes:
//! - bytes 0-7:   `package_file_tag` (u64 LE, constant across chunks)
//! - bytes 8-15:  `max_chunk_size` (u64 LE)
//! - byte 16:     `compression_format` (always 3 = zlib)
//! - bytes 17-24: `compressed_size` (u64 LE; we use the low 32 bits)
//! - bytes 25-32: `uncompressed_size` (u64 LE)
//! - bytes 33-40: `compressed_size_2` (duplicate)
//! - bytes 41-48: `uncompressed_size_2` (duplicate)
//!
//! For `save_version < 41` the header is 48 bytes with no compression-format byte;
//! that path is NOT supported yet — see plan §1.

use crate::error::{Error, Result};
use crate::reader::Reader;

pub const CHUNK_HEADER_LEN_V41: usize = 49;
pub const COMPRESSION_FORMAT_ZLIB: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    pub package_file_tag: u64,
    pub max_chunk_size: u64,
    pub compression_format: u8,
    pub compressed_size: u32,
    pub uncompressed_size: u64,
}

/// Read a chunk header from the current position of `r`.
/// Only `save_version >= 41` (49-byte format) is supported.
pub fn read_chunk_header(r: &mut Reader<'_>, save_version: i32) -> Result<ChunkHeader> {
    if save_version < 41 {
        return Err(Error::UnsupportedSaveVersion {
            found: save_version,
        });
    }
    let bytes: [u8; CHUNK_HEADER_LEN_V41] = r.read_array()?;

    let package_file_tag = u64::from_le_bytes(bytes[0..8].try_into().expect("8-byte slice"));
    let max_chunk_size = u64::from_le_bytes(bytes[8..16].try_into().expect("8-byte slice"));
    let compression_format = bytes[16];
    if compression_format != COMPRESSION_FORMAT_ZLIB {
        return Err(Error::UnsupportedCompressionFormat {
            found: compression_format,
        });
    }
    // NOTE: compressed_size on disk is a u64 (bytes 17-24) but we read only the
    // low 32 bits. Satisfactory caps individual chunks at `max_chunk_size`
    // (typically 0x20000 = 128 KB), so the high 32 bits are always zero in
    // practice. If a future game version ever emits chunks > 4 GB, this read
    // will silently truncate — revisit then.
    let compressed_size = u32::from_le_bytes(bytes[17..21].try_into().expect("4-byte slice"));
    let uncompressed_size = u64::from_le_bytes(bytes[25..33].try_into().expect("8-byte slice"));

    Ok(ChunkHeader {
        package_file_tag,
        max_chunk_size,
        compression_format,
        compressed_size,
        uncompressed_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic 49-byte chunk header for `save_version` >= 41.
    fn synth_chunk_header_v41(
        package_tag: u64,
        max_chunk: u64,
        compressed: u32,
        uncompressed: u64,
    ) -> Vec<u8> {
        let mut b = Vec::with_capacity(CHUNK_HEADER_LEN_V41);
        b.extend_from_slice(&package_tag.to_le_bytes());
        b.extend_from_slice(&max_chunk.to_le_bytes());
        b.push(COMPRESSION_FORMAT_ZLIB);
        // compressed_size as u64 LE; only low 32 bits matter to the parser
        b.extend_from_slice(&u64::from(compressed).to_le_bytes());
        b.extend_from_slice(&uncompressed.to_le_bytes());
        // Duplicate compressed + uncompressed (the parser ignores these)
        b.extend_from_slice(&u64::from(compressed).to_le_bytes());
        b.extend_from_slice(&uncompressed.to_le_bytes());
        assert_eq!(b.len(), CHUNK_HEADER_LEN_V41);
        b
    }

    #[test]
    fn read_chunk_header_v41_parses_all_fields() {
        let bytes = synth_chunk_header_v41(0x9E2A_83C1_u64, 0x20000, 12_345, 131_072);
        let mut r = Reader::new(&bytes);
        let h = read_chunk_header(&mut r, 46).unwrap();
        assert_eq!(h.package_file_tag, 0x9E2A_83C1_u64);
        assert_eq!(h.max_chunk_size, 0x20000);
        assert_eq!(h.compression_format, COMPRESSION_FORMAT_ZLIB);
        assert_eq!(h.compressed_size, 12_345);
        assert_eq!(h.uncompressed_size, 131_072);
        assert_eq!(r.position(), CHUNK_HEADER_LEN_V41);
    }

    #[test]
    fn read_chunk_header_rejects_old_save_version() {
        let bytes = vec![0_u8; CHUNK_HEADER_LEN_V41];
        let mut r = Reader::new(&bytes);
        let err = read_chunk_header(&mut r, 40).unwrap_err();
        assert!(matches!(err, Error::UnsupportedSaveVersion { found: 40 }));
        // The reader should NOT have advanced when the save_version was rejected
        assert_eq!(r.position(), 0);
    }

    #[test]
    fn read_chunk_header_truncated_input() {
        let bytes = vec![0_u8; 20]; // way too short
        let mut r = Reader::new(&bytes);
        let err = read_chunk_header(&mut r, 46).unwrap_err();
        assert!(matches!(
            err,
            Error::UnexpectedEof {
                wanted: 49,
                available: 20,
                at: 0
            }
        ));
    }

    #[test]
    fn read_chunk_header_rejects_non_zlib_compression() {
        // Build a chunk header with a bogus compression_format byte
        let mut b = synth_chunk_header_v41(0x9E2A_83C1_u64, 0x20000, 12, 24);
        b[16] = 0xFF;
        let mut r = Reader::new(&b);
        let err = read_chunk_header(&mut r, 46).unwrap_err();
        assert!(matches!(err, Error::UnsupportedCompressionFormat { found: 0xFF }));
    }
}
