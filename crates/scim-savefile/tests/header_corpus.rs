//! Integration test: parse pinned `.sav` fixtures and assert the header decodes.
//!
//! Add a new fn here for each fixture in `tests/corpus/`.

use scim_savefile::read_header;
use scim_savefile::versions::{MAX_KNOWN_HEADER_TYPE, MIN_SUPPORTED_HEADER_TYPE};
use std::path::PathBuf;

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_header_parses() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav"))
        .expect("missing fixture: tests/corpus/CREATIVE TEST.sav");
    let (header, consumed) = read_header(&bytes).expect("header should parse");

    // Spec invariants (not specific to this fixture):
    assert!(
        header.save_header_type >= MIN_SUPPORTED_HEADER_TYPE,
        "header type too old: {}",
        header.save_header_type
    );
    assert!(
        header.save_header_type <= MAX_KNOWN_HEADER_TYPE,
        "header type newer than known: {}",
        header.save_header_type
    );
    assert!(header.save_version > 0);
    assert!(header.build_version > 0);
    assert!(!header.map_name.is_empty(), "map_name should not be empty");
    assert!(
        consumed > 0 && consumed < bytes.len(),
        "header should consume some bytes but leave the body for later phases"
    );

    // Print so it goes into test output when run with --nocapture; helps populate the corpus README.
    eprintln!(
        "CREATIVE TEST.sav header_type={} save_version={} build_version={} map={:?} session={:?}",
        header.save_header_type,
        header.save_version,
        header.build_version,
        header.map_name,
        header.session_name
    );
}
