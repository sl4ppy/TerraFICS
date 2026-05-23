//! Bench: `read_header` against CREATIVE TEST.sav (the only fixture we ship).

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_savefile::read_header;

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_read_header(c: &mut Criterion) {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav"))
        .expect("CREATIVE TEST.sav must exist in tests/corpus/");
    c.bench_function("read_header", |b| {
        b.iter(|| {
            let _ = read_header(black_box(&bytes)).expect("header parses");
        });
    });
}

criterion_group!(benches, bench_read_header);
criterion_main!(benches);
