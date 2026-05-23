//! Smoke binary: classify a `.sav` file's actors and dispatch typed Components.
//!
//! Usage:
//!     cargo run -p scim-model --example classify-header -- path\to\save.sav

use std::path::PathBuf;
use std::process::ExitCode;

use scim_model::{ClassKind, Registry, TypedComponent};
use scim_savefile::{
    parse_entity_body, read_body, read_body_envelope, read_header, stream_actors,
};

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: classify-header <path-to-sav>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(path);

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };

    let (h, consumed) = match read_header(&bytes) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: parse header: {e}");
            return ExitCode::from(1);
        }
    };
    let body = match read_body(&bytes[consumed..], h.save_version) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: body: {e}");
            return ExitCode::from(1);
        }
    };
    let env = match read_body_envelope(&body, &h) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: envelope: {e}");
            return ExitCode::from(1);
        }
    };

    let registry = Registry::new();
    let mut by_kind: std::collections::HashMap<ClassKind, usize> = std::collections::HashMap::new();
    let mut decoded: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();

    for r in stream_actors(&env, &h) {
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
            .map_or(h.save_version, |l| l.save_version);
        let Ok(eb) = parse_entity_body(&actor, level_save_version, 1000, &h.map_name) else {
            continue;
        };
        if let Ok(Some(typed)) = registry.decode_for_actor(&actor, &eb) {
            let v = match typed {
                TypedComponent::ConveyorBelt(_) => "ConveyorBelt",
                TypedComponent::ConveyorChainActor(_) => "ConveyorChainActor",
                TypedComponent::Splitter(_) => "Splitter",
                TypedComponent::Miner(_) => "Miner",
                TypedComponent::Pipeline(_) => "Pipeline",
                TypedComponent::ResourceNode(_) => "ResourceNode",
            };
            *decoded.entry(v).or_default() += 1;
        }
    }

    let mut sorted: Vec<_> = by_kind.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    println!("File: {}", path.display());
    println!("Classification:");
    for (k, c) in sorted {
        println!("  {k:?}: {c}");
    }
    println!("Typed components decoded:");
    let mut dec: Vec<_> = decoded.iter().collect();
    dec.sort_by(|a, b| b.1.cmp(a.1));
    for (v, c) in dec {
        println!("  {v}: {c}");
    }

    ExitCode::SUCCESS
}
