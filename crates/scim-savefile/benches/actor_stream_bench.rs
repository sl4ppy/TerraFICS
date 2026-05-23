//! Bench: full `stream_actors` iteration (count actors only, no body parsing).

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_savefile::{read_body, read_body_envelope, read_header, stream_actors};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_stream_actors(c: &mut Criterion) {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("fixture present");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    let env = read_body_envelope(&body, &header).expect("envelope");
    c.bench_function("stream_actors[CREATIVE TEST.sav]", |b| {
        b.iter(|| {
            let count = stream_actors(black_box(&env), black_box(&header))
                .flatten()
                .count();
            black_box(count);
        });
    });
}

criterion_group!(benches, bench_stream_actors);
criterion_main!(benches);
