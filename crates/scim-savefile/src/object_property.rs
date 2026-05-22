//! UE "`ObjectProperty`" — a level/path reference pair used in many places:
//! object headers, entity references, paste data, collectables.
//!
//! Wire format (variable length): one or two UE strings.
//! - Read string A.
//! - If A equals `header.map_name`, then A is actually the pathName itself
//!   (no levelName); STOP.
//! - Otherwise A is the levelName; read string B as the pathName.
//!
//! Cross-reference: SC-InteractiveMap/src/SaveParser/Read.js:2476-2490.

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
    if first == map_name {
        // Collapsed form: first WAS the pathName; no level prefix.
        Ok(ObjectProperty {
            level_name: None,
            path_name: first,
        })
    } else {
        let path_name = r.read_string()?;
        Ok(ObjectProperty {
            level_name: Some(first),
            path_name,
        })
    }
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
        let mut bytes = Vec::new();
        write_ascii(&mut bytes, "MapName");
        let mut r = Reader::new(&bytes);
        let p = read_object_property(&mut r, "MapName").unwrap();
        assert_eq!(p.level_name, None);
        assert_eq!(p.path_name, "MapName");
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
