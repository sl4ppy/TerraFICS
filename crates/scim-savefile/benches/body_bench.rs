//! Bench: `read_body` (parallel zlib chunk decompression).

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_savefile::{read_body, read_header};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_read_body(c: &mut Criterion) {
    let bytes = std::fs::read(corpus_path("CREATIVE TEST.sav")).expect("fixture present");
    let (header, consumed) = read_header(&bytes).expect("header");
    let body_bytes = bytes[consumed..].to_vec();
    let save_version = header.save_version;

    c.bench_function("read_body[CREATIVE TEST.sav]", |b| {
        b.iter(|| {
            let _ = read_body(black_box(&body_bytes), black_box(save_version))
                .expect("body decompresses");
        });
    });
}

criterion_group!(benches, bench_read_body);
criterion_main!(benches);
