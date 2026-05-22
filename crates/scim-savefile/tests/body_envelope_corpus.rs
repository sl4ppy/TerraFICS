//! Integration test: walk the body envelope of a real `.sav` fixture.

use std::path::PathBuf;

use scim_savefile::{read_body, read_body_envelope, read_header};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_envelope_walks() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav"))
        .expect("missing fixture: tests/corpus/CREATIVE TEST.sav");

    let (header, consumed) = read_header(&bytes).expect("header should parse");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body should decompress");

    let env = read_body_envelope(&body, &header).expect("envelope should walk");

    // Spec invariants:
    assert!(!env.levels.is_empty(), "expected at least one level");
    // Main level is the last one.
    let main = env.levels.last().unwrap();
    assert_eq!(main.name, format!("Level {}", header.map_name));
    assert_eq!(main.save_version, header.save_version);
    assert!(
        !main.objects_byte_range.is_empty(),
        "main level objects block should not be empty"
    );
    assert!(
        !main.entities_byte_range.is_empty(),
        "main level entities block should not be empty"
    );

    // Every level's byte range must lie within body_bytes.
    for level in &env.levels {
        assert!(
            level.objects_byte_range.end <= env.body_bytes.len(),
            "objects range overflows body for level {}",
            level.name
        );
        assert!(
            level.entities_byte_range.end <= env.body_bytes.len(),
            "entities range overflows body for level {}",
            level.name
        );
    }

    // Print a summary for human inspection (visible with --nocapture).
    eprintln!(
        "CREATIVE TEST.sav: {} level(s), partitions={}",
        env.levels.len(),
        env.partitions.is_some(),
    );
    for level in &env.levels {
        eprintln!(
            "  - {} (save_version={}): objects={} B, entities={} B",
            level.name,
            level.save_version,
            level.objects_byte_range.len(),
            level.entities_byte_range.len(),
        );
    }
}
