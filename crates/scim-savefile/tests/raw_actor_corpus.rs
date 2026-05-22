//! Integration test: stream actors from CREATIVE TEST.sav and assert the totals.

use std::path::PathBuf;

use scim_savefile::object_header::ObjectKind;
use scim_savefile::{read_body, read_body_envelope, read_header, stream_actors};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_streams_actors() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("missing fixture");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");

    let mut total = 0_usize;
    let mut total_object = 0_usize;
    let mut total_actor = 0_usize;
    let mut failures: Vec<String> = Vec::new();
    let mut per_level_main: usize = 0;

    for r in stream_actors(&env, &header) {
        match r {
            Ok(a) => {
                total += 1;
                match a.header.kind {
                    ObjectKind::Object => total_object += 1,
                    ObjectKind::Actor => total_actor += 1,
                }
                if a.level_name == format!("Level {}", header.map_name) {
                    per_level_main += 1;
                }
            }
            Err(e) => failures.push(e.to_string()),
        }
    }

    eprintln!(
        "CREATIVE TEST.sav: {total} actors total ({total_object} objects + {total_actor} actors)"
    );
    eprintln!("  main level: {per_level_main} actors");
    if !failures.is_empty() {
        eprintln!("  failures: {failures:?}");
    }

    assert!(failures.is_empty(), "{} parse failures", failures.len());
    assert!(
        total > 1000,
        "expected > 1000 actors in a real save, got {total}"
    );
    assert!(per_level_main > 0, "main level should contain actors");
}
