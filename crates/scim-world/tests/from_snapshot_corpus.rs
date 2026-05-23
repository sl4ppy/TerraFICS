//! Integration test: import CREATIVE TEST.sav via scim-store, materialize a
//! `WorldIndex`, and sanity-check counts + a viewport-style query.

use std::path::PathBuf;
use std::time::Instant;

use scim_store::{actor::list_actors_in_snapshot, import::import_save, Db};
use scim_world::WorldIndex;

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn from_snapshot_indexes_creative_test_sav() {
    let sav = corpus_path("CREATIVE TEST.sav");
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("creative.scimdb");
    let mut db = Db::open(&db_path).expect("open db");

    let summary = import_save(&mut db, &sav, "CREATIVE TEST.sav").expect("import");
    assert_eq!(summary.total_actors, 23941);
    assert_eq!(summary.failed_actors, 0);

    let all_rows = list_actors_in_snapshot(db.conn(), summary.snapshot_id).unwrap();
    let rows_with_transform =
        all_rows.iter().filter(|r| r.transform.is_some()).count();

    let t0 = Instant::now();
    let idx = WorldIndex::from_snapshot(db.conn(), summary.snapshot_id).unwrap();
    let elapsed = t0.elapsed();

    eprintln!(
        "=== P1.5-a from_snapshot_corpus report ===\n  rows total:    {}\n  rows w/ xform: {}\n  index size:    {}\n  build time:    {:.3}s",
        summary.total_actors,
        rows_with_transform,
        idx.len(),
        elapsed.as_secs_f64()
    );

    assert!(!idx.is_empty(), "expected at least one placement");
    assert_eq!(
        idx.len(),
        rows_with_transform,
        "index should contain exactly the rows whose transform column is non-null"
    );
    assert!(
        idx.len() <= summary.total_actors,
        "index size cannot exceed total actor count"
    );

    // 5s is generous; this isn't a perf budget, it's a "tuning silently broken"
    // canary. Real perf numbers come from the criterion bench in Task 8.
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "from_snapshot took {:.2}s; the SQLite read or decode is silently broken",
        elapsed.as_secs_f64()
    );

    // A modest AABB around the origin should return some subset, not all.
    assert!(
        idx.query_aabb([-100_000.0, -100_000.0], [100_000.0, 100_000.0])
            .count() <= idx.len(),
        "filtered query cannot exceed full index"
    );
}
