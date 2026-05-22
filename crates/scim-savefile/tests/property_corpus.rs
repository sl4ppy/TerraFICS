//! Integration test: parse property bags of every actor in CREATIVE TEST.sav.
//! Reports a histogram of property type frequencies and unsupported-type counts.

use std::collections::HashMap;
use std::path::PathBuf;

use scim_savefile::{parse_entity_body, read_body, read_body_envelope, read_header, stream_actors};

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

    // After P1.3-b, ≥ 95% of actors should fully parse. A few hard errors are tolerated
    // (mod-specific structs where the nested-property fallback over-consumes and can't be
    // recovered as an OpaqueBlob); P1.3-c will close these via typed decoders.
    #[allow(clippy::cast_precision_loss)]
    let percent_full = (fully_parsed as f64 / total_actors as f64) * 100.0;
    assert!(
        percent_full >= 95.0,
        "expected ≥ 95% of actors to fully parse after P1.3-b, got {percent_full:.1}% ({fully_parsed}/{total_actors})"
    );
    assert!(
        hard_errors < total_actors / 100,
        "hard errors should stay below 1% of actors; got {hard_errors}/{total_actors}"
    );
    assert!(
        total_properties > 15_000,
        "expected > 15k decoded properties after P1.3-b, got {total_properties}"
    );
}
