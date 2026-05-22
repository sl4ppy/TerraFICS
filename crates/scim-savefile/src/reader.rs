//! Byte cursor for parsing UE-style binary files.
//! All multi-byte integers are little-endian.

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    #[must_use]
    pub const fn position(&self) -> usize { self.pos }
    #[must_use]
    pub const fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.pos) }

    #[inline]
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let available = self.bytes.len() - self.pos;
        if n > available {
            return Err(Error::UnexpectedEof {
                wanted: n,
                available,
                at: self.pos,
            });
        }
        let out = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        let b = self.take(8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(b);
        Ok(i64::from_le_bytes(buf))
    }

    pub fn read_hex(&mut self, n: usize) -> Result<Vec<u8>> {
        Ok(self.take(n)?.to_vec())
    }

    /// UE-style length-prefixed string.
    /// - 0   => empty string
    /// - >0  => ASCII/UTF-8; length includes optional null terminator (stripped if present)
    /// - <0  => UTF-16-LE; abs(length) is char count including optional null terminator
    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_i32()?;
        match len.cmp(&0) {
            std::cmp::Ordering::Equal => Ok(String::new()),
            std::cmp::Ordering::Greater => {
                let n = len.unsigned_abs() as usize;
                let at = self.pos;
                let bytes = self.take(n)?;
                let end = if !bytes.is_empty() && bytes[n - 1] == 0 { n - 1 } else { n };
                std::str::from_utf8(&bytes[..end])
                    .map(str::to_owned)
                    .map_err(|source| Error::InvalidUtf8 { at, source })
            }
            std::cmp::Ordering::Less => {
                let n = len.unsigned_abs() as usize;
                // Defend against length prefixes that would overflow usize when doubled,
                // and skip the take() in the obvious-EOF case to give a precise error.
                if n > self.remaining() / 2 {
                    return Err(Error::UnexpectedEof {
                        wanted: n.saturating_mul(2),
                        available: self.remaining(),
                        at: self.pos,
                    });
                }
                let byte_len = n * 2;
                let at = self.pos;
                let bytes = self.take(byte_len)?;
                let mut units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                if units.last() == Some(&0) { units.pop(); }
                String::from_utf16(&units).map_err(|_| Error::InvalidUtf16 { at })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_u8_advances_one_byte() {
        let mut r = Reader::new(&[0x42, 0x99]);
        assert_eq!(r.read_u8().unwrap(), 0x42);
        assert_eq!(r.position(), 1);
        assert_eq!(r.read_u8().unwrap(), 0x99);
    }

    #[test]
    fn read_u8_at_eof_errors() {
        let mut r = Reader::new(&[]);
        let err = r.read_u8().unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { wanted: 1, available: 0, at: 0 }));
    }

    #[test]
    fn read_i32_is_little_endian() {
        // 0x12345678 in little-endian bytes
        let mut r = Reader::new(&[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(r.read_i32().unwrap(), 0x1234_5678);
        assert_eq!(r.position(), 4);
    }

    #[test]
    fn read_i32_handles_negative() {
        // -1 in two's complement little-endian
        let mut r = Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(r.read_i32().unwrap(), -1);
    }

    #[test]
    fn read_i64_is_little_endian() {
        let mut r = Reader::new(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(r.read_i64().unwrap(), 0x0807_0605_0403_0201);
    }

    #[test]
    fn read_hex_returns_exact_slice_as_vec() {
        let mut r = Reader::new(&[0xDE, 0xAD, 0xBE, 0xEF, 0xFF]);
        let v = r.read_hex(4).unwrap();
        assert_eq!(v, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(r.position(), 4);
    }

    #[test]
    fn read_string_ascii() {
        // Length 6 (5 chars + null), then "hello\0"
        let mut bytes = vec![];
        bytes.extend_from_slice(&6_i32.to_le_bytes());
        bytes.extend_from_slice(b"hello\0");
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_string().unwrap(), "hello");
        assert_eq!(r.position(), 10);
    }

    #[test]
    fn read_string_empty() {
        let bytes = 0_i32.to_le_bytes();
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_string().unwrap(), "");
    }

    #[test]
    fn read_string_without_trailing_null_is_accepted() {
        // Some saves omit the null. Length is the full content.
        let mut bytes = vec![];
        bytes.extend_from_slice(&5_i32.to_le_bytes());
        bytes.extend_from_slice(b"hello");
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_string().unwrap(), "hello");
    }

    #[test]
    fn read_string_utf16() {
        // Length -3 means 3 UTF-16 code units (incl. null): "hi\0"
        let mut bytes = vec![];
        bytes.extend_from_slice(&(-3_i32).to_le_bytes());
        bytes.extend_from_slice(&b'h'.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&b'i'.to_le_bytes());
        bytes.push(0);
        bytes.push(0); // null code unit
        bytes.push(0);
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_string().unwrap(), "hi");
    }

    #[test]
    fn read_i32_at_eof_errors() {
        let mut r = Reader::new(&[0x00, 0x00, 0x00]);
        let err = r.read_i32().unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { wanted: 4, available: 3, at: 0 }));
    }

    #[test]
    fn read_i64_at_eof_errors() {
        let mut r = Reader::new(&[0x01, 0x02, 0x03]);
        let err = r.read_i64().unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { wanted: 8, available: 3, at: 0 }));
    }

    #[test]
    fn read_string_invalid_utf8() {
        let mut bytes = vec![];
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        bytes.push(0xFF); // not a valid UTF-8 start byte
        let mut r = Reader::new(&bytes);
        let err = r.read_string().unwrap_err();
        assert!(matches!(err, Error::InvalidUtf8 { .. }));
    }

    #[test]
    fn read_string_utf16_lone_surrogate() {
        // Length -2 => 2 code units. First code unit is a high surrogate (0xD800) with
        // nothing valid after it. Total of 4 bytes.
        let mut bytes = vec![];
        bytes.extend_from_slice(&(-2_i32).to_le_bytes());
        bytes.push(0x00); bytes.push(0xD8); // 0xD800 high surrogate (LE bytes)
        bytes.push(0x00); bytes.push(0x00); // null code unit (stripped before from_utf16)
        let mut r = Reader::new(&bytes);
        let err = r.read_string().unwrap_err();
        assert!(matches!(err, Error::InvalidUtf16 { .. }));
    }

    #[test]
    fn read_string_utf16_eof_on_huge_length() {
        // -1_000_000 chars claimed but no bytes follow. Should error cleanly, not panic
        // or allocate 2 MB+. This exercises the n > remaining()/2 short-circuit.
        let mut bytes = vec![];
        bytes.extend_from_slice(&(-1_000_000_i32).to_le_bytes());
        let mut r = Reader::new(&bytes);
        let err = r.read_string().unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { .. }));
    }
}
