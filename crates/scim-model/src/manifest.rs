//! Mod-manifest TOML format. One `.toml` file per mod (typically), each
//! containing a top-level `mod_id` plus an array of `[[classes]]` entries.
//!
//! Example:
//! ```toml
//! mod_id = "translucid-belts"
//!
//! [[classes]]
//! class_name = "/TranslucidBelts/Build_TB_Mk1.Build_TB_Mk1_C"
//! kind = "ConveyorBelt"
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::classdef::{ClassDef, ClassKind};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub mod_id: String,
    #[serde(default)]
    pub classes: Vec<ModManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifestEntry {
    pub class_name: String,
    pub kind: ClassKind,
}

impl ModManifest {
    /// Convert the manifest's entries into `ClassDef` values tagged with this
    /// manifest's `mod_id`.
    #[must_use]
    pub fn to_class_defs(&self) -> Vec<ClassDef> {
        self.classes
            .iter()
            .map(|e| ClassDef::from_mod(e.class_name.clone(), e.kind, self.mod_id.clone()))
            .collect()
    }
}

/// Parse a single manifest file.
pub fn load_manifest(path: &Path) -> Result<ModManifest> {
    let contents = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| Error::TomlParse {
        path: path.to_path_buf(),
        source,
    })
}

/// Walk `dir` for `*.toml` files and parse each into a `ModManifest`. Returns
/// the manifests in (unspecified) order. Missing or empty directories return
/// `Ok(vec![])`.
pub fn load_manifests_from_dir(dir: &Path) -> Result<Vec<ModManifest>> {
    let mut out = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(source) => {
            return Err(Error::Io {
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    for entry in read_dir {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path: PathBuf = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            out.push(load_manifest(&path)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &std::path::Path, name: &str, contents: &str) -> PathBuf {
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        p
    }

    #[test]
    fn parses_two_class_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(
            dir.path(),
            "translucid.toml",
            r#"
mod_id = "translucid-belts"

[[classes]]
class_name = "/TranslucidBelts/Build_TB_Mk1.Build_TB_Mk1_C"
kind = "ConveyorBelt"

[[classes]]
class_name = "/TranslucidBelts/Build_TB_Mk2.Build_TB_Mk2_C"
kind = "ConveyorBelt"
"#,
        );
        let m = load_manifest(&path).unwrap();
        assert_eq!(m.mod_id, "translucid-belts");
        assert_eq!(m.classes.len(), 2);
        assert_eq!(m.classes[0].kind, ClassKind::ConveyorBelt);
    }

    #[test]
    fn empty_dir_returns_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let manifests = load_manifests_from_dir(dir.path()).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn missing_dir_returns_empty_vec() {
        let path = std::path::PathBuf::from("/this/path/does/not/exist");
        let manifests = load_manifests_from_dir(&path).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn ignores_non_toml_files() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "readme.md", "this is not a manifest");
        let manifests = load_manifests_from_dir(dir.path()).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn malformed_toml_surfaces_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "bad.toml", "mod_id = ");
        let err = load_manifest(&path).unwrap_err();
        assert!(matches!(err, Error::TomlParse { .. }));
    }

    #[test]
    fn to_class_defs_stamps_mod_id() {
        let m = ModManifest {
            mod_id: "abc".to_string(),
            classes: vec![ModManifestEntry {
                class_name: "/X/Y".to_string(),
                kind: ClassKind::ConveyorBelt,
            }],
        };
        let defs = m.to_class_defs();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].mod_origin.as_deref(), Some("abc"));
    }
}
