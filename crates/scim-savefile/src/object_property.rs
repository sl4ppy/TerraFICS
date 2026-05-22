//! UE "`ObjectProperty`" — a level/path reference pair used in many places:
//! object headers, entity references, paste data, collectables.
//!
//! Wire format: ALWAYS two UE strings.
//! - String A is the levelName.
//! - String B is the pathName.
//! - When A equals `header.map_name`, the JS source treats A as a redundant marker
//!   and discards it (`level_name = None`); the actual reference is just the pathName.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2476-2490 — note that
//! BOTH branches of the if/else read a second string, even though the JS variable
//! naming makes it look otherwise.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::reader::Reader;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectProperty {
    /// `None` when the first string equaled `map_name` (collapsed form).
    pub level_name: Option<String>,
    pub path_name: String,
}

pub fn read_object_property(r: &mut Reader<'_>, map_name: &str) -> Result<ObjectProperty> {
    let first = r.read_string()?;
    let path_name = r.read_string()?;
    let level_name = if first == map_name { None } else { Some(first) };
    Ok(ObjectProperty {
        level_name,
        path_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_ascii(out: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        let len = i32::try_from(bytes.len() + 1).expect("string length fits i32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0);
    }

    #[test]
    fn collapsed_form_when_first_equals_map_name() {
        // Both strings are still read; the first (== map_name) is discarded.
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "MapName");
        write_ascii(&mut bytes, "Persistent.Foo_42");
        let mut r = Reader::new(&bytes);
        let p = read_object_property(&mut r, "MapName").unwrap();
        assert_eq!(p.level_name, None);
        assert_eq!(p.path_name, "Persistent.Foo_42");
    }

    #[test]
    fn expanded_form_when_first_is_level_name() {
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "Persistent_Level");
        write_ascii(&mut bytes, "PersistentLevel.Build_Foo_42");
        let mut r = Reader::new(&bytes);
        let p = read_object_property(&mut r, "MapName").unwrap();
        assert_eq!(p.level_name.as_deref(), Some("Persistent_Level"));
        assert_eq!(p.path_name, "PersistentLevel.Build_Foo_42");
    }
}
