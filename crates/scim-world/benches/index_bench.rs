//! Criterion bench: `WorldIndex::from_snapshot` and a representative
//! `query_aabb` over the CREATIVE TEST.sav-derived database.

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_store::{import::import_save, Db};
use scim_world::WorldIndex;

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

fn setup_db() -> (tempfile::TempDir, Db, i64) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bench.scimdb");
    let mut db = Db::open(&path).expect("open db");
    let summary = import_save(&mut db, corpus_path("CREATIVE TEST.sav"), "bench")
        .expect("import");
    (dir, db, summary.snapshot_id)
}

fn bench_from_snapshot(c: &mut Criterion) {
    let (_dir, db, snapshot_id) = setup_db();
    let mut group = c.benchmark_group("scim_world::from_snapshot");
    group.sample_size(20);
    group.bench_function("CREATIVE TEST.sav", |b| {
        b.iter(|| {
            let idx =
                WorldIndex::from_snapshot(db.conn(), black_box(snapshot_id)).unwrap();
            black_box(idx);
        });
    });
    group.finish();
}

fn bench_query_aabb(c: &mut Criterion) {
    let (_dir, db, snapshot_id) = setup_db();
    let idx = WorldIndex::from_snapshot(db.conn(), snapshot_id).unwrap();
    let mut group = c.benchmark_group("scim_world::query_aabb");
    group.bench_function("viewport_100k", |b| {
        b.iter(|| {
            let hits: usize = idx
                .query_aabb(black_box([-50_000.0, -50_000.0]), black_box([50_000.0, 50_000.0]))
                .count();
            black_box(hits);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_from_snapshot, bench_query_aabb);
criterion_main!(benches);
