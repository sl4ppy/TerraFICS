//! Integration test: classify all actors and decode ConveyorBelt/Lift trailing
//! bytes against CREATIVE TEST.sav (lives in the scim-savefile crate's tests dir).

use std::collections::HashMap;
use std::path::PathBuf;

use scim_model::{ClassKind, Component, ConveyorBelt, Registry};
use scim_savefile::{parse_entity_body, read_body, read_body_envelope, read_header, stream_actors};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn creative_test_sav_conveyor_decode() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("missing fixture");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");

    let registry = Registry::new();

    let mut by_kind: HashMap<ClassKind, usize> = HashMap::new();
    let mut conveyor_belts_decoded = 0_usize;
    let mut conveyor_belts_with_items = 0_usize;
    let mut total_items_on_belts = 0_usize;
    let mut decode_failures = 0_usize;
    let mut first_failure: Option<String> = None;

    for r in stream_actors(&env, &header) {
        let Ok(actor) = r else { continue };
        let kind = registry.classify(&actor.header.class_name);
        *by_kind.entry(kind).or_default() += 1;

        if matches!(kind, ClassKind::ConveyorBelt | ClassKind::ConveyorLift) {
            let level_save_version = env
                .levels
                .iter()
                .find(|l| l.name == actor.level_name)
                .map_or(header.save_version, |l| l.save_version);
            let Ok(eb) = parse_entity_body(&actor, level_save_version, 1000, &header.map_name)
            else {
                continue;
            };
            match ConveyorBelt::decode(&actor, &eb) {
                Ok(belt) => {
                    conveyor_belts_decoded += 1;
                    if !belt.items.is_empty() {
                        conveyor_belts_with_items += 1;
                    }
                    total_items_on_belts += belt.items.len();
                }
                Err(e) => {
                    decode_failures += 1;
                    if first_failure.is_none() {
                        first_failure = Some(e.to_string());
                    }
                }
            }
        }
    }

    eprintln!("=== P1.3-c ConveyorBelt corpus report ===");
    let mut by_kind_sorted: Vec<_> = by_kind.iter().collect();
    by_kind_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (k, c) in by_kind_sorted {
        eprintln!("  {k:?}: {c}");
    }
    let belt_total = by_kind.get(&ClassKind::ConveyorBelt).copied().unwrap_or(0)
        + by_kind.get(&ClassKind::ConveyorLift).copied().unwrap_or(0);
    eprintln!("ConveyorBelt/Lift actors:   {belt_total}");
    eprintln!("  decoded:                  {conveyor_belts_decoded}");
    eprintln!("  with at least 1 item:     {conveyor_belts_with_items}");
    eprintln!("  total items on belts:     {total_items_on_belts}");
    eprintln!("  decode failures:          {decode_failures}");
    if let Some(msg) = &first_failure {
        eprintln!("  first failure:            {msg}");
    }

    assert!(
        belt_total > 0,
        "expected at least one ConveyorBelt/Lift actor in the corpus"
    );
    #[allow(clippy::cast_precision_loss)]
    let success_pct = (conveyor_belts_decoded as f64 / belt_total as f64) * 100.0;
    assert!(
        success_pct >= 95.0,
        "expected ≥ 95% of belt/lift actors to decode, got {success_pct:.1}% ({conveyor_belts_decoded}/{belt_total})"
    );
}
