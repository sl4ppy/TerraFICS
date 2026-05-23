//! Bench: parse every actor's property bag through `parse_entity_body`.

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_savefile::{parse_entity_body, read_body, read_body_envelope, read_header, stream_actors};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_parse_entity_body(c: &mut Criterion) {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("fixture present");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");

    c.bench_function("parse_entity_body[CREATIVE TEST.sav, all actors]", |b| {
        b.iter(|| {
            let mut decoded = 0_usize;
            for r in stream_actors(black_box(&env), black_box(&header)) {
                let Ok(actor) = r else { continue };
                let level_save_version = env
                    .levels
                    .iter()
                    .find(|l| l.name == actor.level_name)
                    .map_or(header.save_version, |l| l.save_version);
                if parse_entity_body(&actor, level_save_version, 1000, &header.map_name).is_ok() {
                    decoded += 1;
                }
            }
            black_box(decoded);
        });
    });
}

criterion_group!(benches, bench_parse_entity_body);
criterion_main!(benches);
