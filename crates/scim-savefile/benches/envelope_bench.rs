//! Bench: `read_body_envelope` (level + partition walking).

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_savefile::{read_body, read_body_envelope, read_header};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_read_body_envelope(c: &mut Criterion) {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("fixture present");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body = read_body(&bytes[consumed..], header.save_version).expect("body");
    c.bench_function("read_body_envelope[CREATIVE TEST.sav]", |b| {
        b.iter(|| {
            let _ = read_body_envelope(black_box(&body), black_box(&header)).expect("envelope");
        });
    });
}

criterion_group!(benches, bench_read_body_envelope);
criterion_main!(benches);
