//! Bench: full `import_save` against CREATIVE TEST.sav.

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_store::{import::import_save, Db};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

fn bench_import_save(c: &mut Criterion) {
    let sav = corpus_path("CREATIVE TEST.sav");
    // Configure a longer measurement_time and fewer samples because each import
    // takes ~5 seconds — default criterion settings would explode wall time.
    let mut group = c.benchmark_group("import_save");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(60));
    group.bench_function("CREATIVE TEST.sav", |b| {
        b.iter_with_setup(
            || {
                let dir = tempfile::tempdir().expect("tempdir");
                let path = dir.path().join("bench.scimdb");
                (dir, path)
            },
            |(dir, path)| {
                let mut db = Db::open(&path).expect("open db");
                let summary = import_save(&mut db, black_box(&sav), "bench").expect("import");
                black_box(summary);
                drop(dir);
            },
        );
    });
    group.finish();
}

criterion_group!(benches, bench_import_save);
criterion_main!(benches);
