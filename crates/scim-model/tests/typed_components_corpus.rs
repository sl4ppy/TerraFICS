//! Integration test: dispatch every actor in CREATIVE TEST.sav through
//! `Registry::decode_for_actor` and report a per-component success histogram.

use std::collections::HashMap;
use std::path::PathBuf;

use scim_model::{ClassKind, Registry, TypedComponent};
use scim_savefile::{
    parse_entity_body, read_body, read_body_envelope, read_header, stream_actors,
};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

const fn variant_name(t: &TypedComponent) -> &'static str {
    match t {
        TypedComponent::ConveyorBelt(_) => "ConveyorBelt",
        TypedComponent::ConveyorChainActor(_) => "ConveyorChainActor",
        TypedComponent::Splitter(_) => "Splitter",
        TypedComponent::Miner(_) => "Miner",
        TypedComponent::Pipeline(_) => "Pipeline",
        TypedComponent::ResourceNode(_) => "ResourceNode",
    }
}

#[test]
fn creative_test_sav_all_typed_components_decode() {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("missing fixture");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");
    let registry = Registry::new();

    let mut by_kind: HashMap<ClassKind, usize> = HashMap::new();
    let mut decoded_by_variant: HashMap<&'static str, usize> = HashMap::new();
    let mut decode_failures: HashMap<&'static str, usize> = HashMap::new();
    let mut first_failure: Option<String> = None;

    for r in stream_actors(&env, &header) {
        let Ok(actor) = r else { continue };
        let kind = registry.classify(&actor.header.class_name);
        *by_kind.entry(kind).or_default() += 1;
        if kind == ClassKind::Unknown {
            continue;
        }
        let level_save_version = env
            .levels
            .iter()
            .find(|l| l.name == actor.level_name)
            .map_or(header.save_version, |l| l.save_version);
        let Ok(eb) = parse_entity_body(&actor, level_save_version, 1000, &header.map_name) else {
            continue;
        };
        match registry.decode_for_actor(&actor, &eb) {
            Ok(Some(typed)) => {
                *decoded_by_variant.entry(variant_name(&typed)).or_default() += 1;
            }
            Ok(None) => {}
            Err(e) => {
                let variant = match kind {
                    ClassKind::ConveyorBelt | ClassKind::ConveyorLift => "ConveyorBelt",
                    ClassKind::ConveyorChainActor => "ConveyorChainActor",
                    ClassKind::Splitter => "Splitter",
                    ClassKind::Miner => "Miner",
                    ClassKind::Pipeline => "Pipeline",
                    ClassKind::ResourceNode => "ResourceNode",
                    ClassKind::Unknown => "Unknown",
                };
                *decode_failures.entry(variant).or_default() += 1;
                if first_failure.is_none() {
                    first_failure = Some(format!("{variant}: {e}"));
                }
            }
        }
    }

    eprintln!("=== P1.3-d typed-components corpus report ===");
    let mut by_kind_sorted: Vec<_> = by_kind.iter().collect();
    by_kind_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (k, c) in by_kind_sorted {
        eprintln!("  classify {k:?}: {c}");
    }
    eprintln!("--- Decoded by Component variant ---");
    let mut dec_sorted: Vec<_> = decoded_by_variant.iter().collect();
    dec_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (v, c) in dec_sorted {
        eprintln!("  {v}: {c}");
    }
    if !decode_failures.is_empty() {
        eprintln!("--- Decode failures ---");
        for (v, c) in &decode_failures {
            eprintln!("  {v}: {c}");
        }
        if let Some(msg) = &first_failure {
            eprintln!("  first failure: {msg}");
        }
    }

    let chain_actors = by_kind
        .get(&ClassKind::ConveyorChainActor)
        .copied()
        .unwrap_or(0);
    let chain_decoded = decoded_by_variant
        .get("ConveyorChainActor")
        .copied()
        .unwrap_or(0);
    assert!(
        chain_actors > 0,
        "expected at least one ConveyorChainActor in the corpus"
    );
    assert_eq!(
        chain_decoded, chain_actors,
        "all ConveyorChainActor actors should decode"
    );

    for (kind, variant) in [
        (ClassKind::Splitter, "Splitter"),
        (ClassKind::Miner, "Miner"),
        (ClassKind::Pipeline, "Pipeline"),
        (ClassKind::ResourceNode, "ResourceNode"),
    ] {
        let total = by_kind.get(&kind).copied().unwrap_or(0);
        let decoded = decoded_by_variant.get(variant).copied().unwrap_or(0);
        if total > 0 {
            assert_eq!(decoded, total, "all {variant} actors should decode");
        }
    }
}
