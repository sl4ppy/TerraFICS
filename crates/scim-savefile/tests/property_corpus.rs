//! Integration test: parse property bags of every actor in CREATIVE TEST.sav.
//! Reports a histogram of property type frequencies and unsupported-type counts.

use std::collections::HashMap;
use std::path::PathBuf;

use scim_savefile::{
    parse_entity_body, read_body, read_body_envelope, read_header, stream_actors,
};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_property_histogram() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("missing fixture");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");

    let mut total_actors = 0_usize;
    let mut fully_parsed = 0_usize;
    let mut stopped_on_unsupported = 0_usize;
    let mut hard_errors = 0_usize;
    let mut total_properties = 0_usize;
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut unsupported_by_type: HashMap<String, usize> = HashMap::new();

    let main_save_version = env
        .levels
        .last()
        .map_or(header.save_version, |l| l.save_version);

    for r in stream_actors(&env, &header) {
        let Ok(actor) = r else {
            hard_errors += 1;
            continue;
        };
        total_actors += 1;

        let level_save_version = env
            .levels
            .iter()
            .find(|l| l.name == actor.level_name)
            .map_or(main_save_version, |l| l.save_version);

        match parse_entity_body(&actor, level_save_version, 1000, &header.map_name) {
            Ok(eb) => {
                total_properties += eb.properties.len();
                for p in &eb.properties {
                    *by_type.entry(p.type_name.clone()).or_default() += 1;
                }
                if let Some(hit) = eb.first_unsupported {
                    stopped_on_unsupported += 1;
                    *unsupported_by_type.entry(hit.type_name).or_default() += 1;
                } else {
                    fully_parsed += 1;
                }
            }
            Err(_) => hard_errors += 1,
        }
    }

    eprintln!("=== P1.3-a property corpus report ===");
    eprintln!("Total actors:          {total_actors}");
    eprintln!("Fully parsed:          {fully_parsed}");
    eprintln!("Stopped on unsupp.:    {stopped_on_unsupported}");
    eprintln!("Hard errors:           {hard_errors}");
    eprintln!("Total properties:      {total_properties}");
    let mut by_type_sorted: Vec<_> = by_type.iter().collect();
    by_type_sorted.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("--- Properties decoded by type ---");
    for (t, c) in &by_type_sorted {
        eprintln!("  {t:<16} {c}");
    }
    let mut unsupp_sorted: Vec<_> = unsupported_by_type.iter().collect();
    unsupp_sorted.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("--- Unsupported types blocking iteration ---");
    for (t, c) in &unsupp_sorted {
        eprintln!("  {t:<16} {c}");
    }

    assert_eq!(
        hard_errors, 0,
        "no hard parse errors expected (only graceful unsupported-stop)"
    );
    assert!(
        total_properties > 1000,
        "expected > 1000 decoded primitive properties, got {total_properties}"
    );
    let any_property_actors = fully_parsed + stopped_on_unsupported;
    assert!(
        any_property_actors > total_actors / 10,
        "expected at least 10% of actors to yield at least one parsed property; got {any_property_actors}/{total_actors}"
    );
}
