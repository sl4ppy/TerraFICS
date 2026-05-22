//! `PropertyGuid` — a 1-or-17-byte preamble that appears before the value of
//! most primitive property types.
//!
//! Wire format:
//! - 1 byte indicator
//! - If indicator == 1, 16 bytes of GUID follow.
//! - Otherwise no GUID bytes.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2492-2501.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::reader::Reader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyGuid {
    /// 16-byte UE GUID. `None` when the indicator byte was 0.
    pub bytes: Option<[u8; 16]>,
}

pub fn read_property_guid(r: &mut Reader<'_>) -> Result<PropertyGuid> {
    let indicator = r.read_u8()?;
    let bytes = if indicator == 1 {
        Some(r.read_array::<16>()?)
    } else {
        None
    };
    Ok(PropertyGuid { bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_guid_when_indicator_is_zero() {
        let mut r = Reader::new(&[0_u8]);
        let g = read_property_guid(&mut r).unwrap();
        assert_eq!(g.bytes, None);
        assert_eq!(r.position(), 1);
    }

    #[test]
    fn reads_guid_when_indicator_is_one() {
        let mut bytes = vec![1_u8];
        bytes.extend_from_slice(&[0xAA; 16]);
        let mut r = Reader::new(&bytes);
        let g = read_property_guid(&mut r).unwrap();
        assert_eq!(g.bytes, Some([0xAA; 16]));
        assert_eq!(r.position(), 17);
    }

    #[test]
    fn other_indicator_values_treated_as_no_guid() {
        // The JS source only reads the GUID when indicator == 1; any other byte is "no GUID".
        let mut r = Reader::new(&[2_u8]);
        let g = read_property_guid(&mut r).unwrap();
        assert_eq!(g.bytes, None);
    }
}
