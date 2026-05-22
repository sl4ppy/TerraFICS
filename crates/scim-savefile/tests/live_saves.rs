//! Integration test against live save files dropped into `test-saves/` at the repo root.
//!
//! This test discovers every `.sav` file in `test-saves/` and parses its header.
//! For saves with `save_version >= 41` it also decompresses the body and asserts
//! the result is non-empty with a plausible leading length prefix.
//!
//! If `test-saves/` is empty (the default), the test no-ops and passes.
//! Run with `--nocapture` to see per-file summaries.

use std::fs;
use std::path::{Path, PathBuf};

use scim_savefile::versions::{MAX_KNOWN_HEADER_TYPE, MIN_SUPPORTED_HEADER_TYPE};
use scim_savefile::{read_body, read_header};

/// `test-saves/` lives at the repo root, two levels up from `crates/scim-savefile/`.
fn live_saves_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-saves")
}

fn collect_sav_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
        })
        .collect()
}

#[test]
fn parse_every_live_save_in_test_saves_dir() {
    let dir = live_saves_dir();
    let saves = collect_sav_files(&dir);

    if saves.is_empty() {
        eprintln!(
            "live_saves: no .sav files in {}; skipping (drop a save file there to enable this test)",
            dir.display()
        );
        return;
    }

    eprintln!(
        "live_saves: found {} .sav file(s) in {}",
        saves.len(),
        dir.display()
    );

    let mut parsed = 0_usize;
    let mut skipped_old_format = 0_usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for path in &saves {
        match parse_one(path) {
            Ok(Outcome::Parsed) => parsed += 1,
            Ok(Outcome::SkippedOldFormat { save_version }) => {
                eprintln!(
                    "  - {}: save_version={save_version} is < 41 (pre-Update-8 format); body decompression skipped",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                skipped_old_format += 1;
            }
            Err(e) => {
                eprintln!(
                    "  - {}: FAILED — {e}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                failures.push((path.clone(), e));
            }
        }
    }

    eprintln!(
        "live_saves: {parsed} parsed cleanly, {skipped_old_format} skipped (old format), {} failures",
        failures.len()
    );

    assert!(
        failures.is_empty(),
        "{} live save(s) failed to parse; see stderr for details",
        failures.len()
    );
}

enum Outcome {
    Parsed,
    SkippedOldFormat { save_version: i32 },
}

#[allow(clippy::too_many_lines)] // sequential diagnostic pipeline; splitting would obscure flow
fn parse_one(path: &Path) -> Result<Outcome, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;

    let (header, consumed) = read_header(&bytes).map_err(|e| format!("header parse: {e}"))?;

    if header.save_header_type < MIN_SUPPORTED_HEADER_TYPE
        || header.save_header_type > MAX_KNOWN_HEADER_TYPE
    {
        return Err(format!(
            "save_header_type {} outside supported range {}..={}",
            header.save_header_type, MIN_SUPPORTED_HEADER_TYPE, MAX_KNOWN_HEADER_TYPE
        ));
    }

    // Print a one-line per-file summary.
    eprintln!(
        "  - {}: header_type={} save_version={} build_version={} session={:?} ({} bytes header)",
        path.file_name().unwrap_or_default().to_string_lossy(),
        header.save_header_type,
        header.save_version,
        header.build_version,
        header.session_name,
        consumed,
    );

    if header.save_version < 41 {
        return Ok(Outcome::SkippedOldFormat {
            save_version: header.save_version,
        });
    }

    let body = read_body(&bytes[consumed..], header.save_version)
        .map_err(|e| format!("body decompress: {e}"))?;

    if body.is_empty() {
        return Err("decompressed body is empty".to_owned());
    }
    if body.len() < 8 {
        return Err(format!(
            "body too short for length prefix: {} bytes",
            body.len()
        ));
    }

    let prefix = u64::from_le_bytes(body[..8].try_into().unwrap());
    let body_len_u64 = u64::try_from(body.len()).expect("body.len() fits u64");
    if prefix == 0 || prefix >= body_len_u64 {
        return Err(format!(
            "implausible body length prefix: {prefix} (body is {} bytes)",
            body.len()
        ));
    }

    let compressed = bytes.len() - consumed;
    #[allow(clippy::cast_precision_loss)] // display-only ratio
    let ratio = body.len() as f64 / compressed.max(1) as f64;
    eprintln!(
        "      body: {compressed} compressed -> {} decompressed ({ratio:.2}x)",
        body.len()
    );

    // Walk the body envelope (P1.2-b1).
    match scim_savefile::read_body_envelope(&body, &header) {
        Ok(env) => {
            eprintln!(
                "      envelope: {} level(s), partitions={}",
                env.levels.len(),
                env.partitions.is_some(),
            );
            for level in &env.levels {
                eprintln!(
                    "        - {} (save_version={}): objects={} B, entities={} B",
                    level.name,
                    level.save_version,
                    level.objects_byte_range.len(),
                    level.entities_byte_range.len(),
                );
            }
        }
        Err(scim_savefile::Error::UnsupportedSaveVersion { found }) => {
            eprintln!("      envelope: skipped (unsupported save_version {found})");
        }
        Err(e) => {
            return Err(format!("envelope walk: {e}"));
        }
    }

    // Stream actors (P1.2-b2).
    match scim_savefile::read_body_envelope(&body, &header) {
        Ok(env) => {
            let mut total = 0_usize;
            let mut actor_count = 0_usize;
            let mut object_count = 0_usize;
            let mut first_failure: Option<String> = None;
            for r in scim_savefile::stream_actors(&env, &header) {
                match r {
                    Ok(a) => {
                        total += 1;
                        match a.header.kind {
                            scim_savefile::object_header::ObjectKind::Object => object_count += 1,
                            scim_savefile::object_header::ObjectKind::Actor => actor_count += 1,
                        }
                    }
                    Err(e) => {
                        if first_failure.is_none() {
                            first_failure = Some(e.to_string());
                        }
                    }
                }
            }
            eprintln!(
                "      actors: {total} total ({object_count} objects + {actor_count} actors)"
            );
            if let Some(msg) = first_failure {
                eprintln!("      FIRST FAILURE: {msg}");
                return Err(format!("actor stream had failures (first: {msg})"));
            }
        }
        Err(scim_savefile::Error::UnsupportedSaveVersion { .. }) => {
            // Already reported by the envelope walk above.
        }
        Err(e) => {
            return Err(format!("re-envelope for actor stream: {e}"));
        }
    }

    Ok(Outcome::Parsed)
}
