//! Integration test: import CREATIVE TEST.sav into a fresh project DB,
//! verify the expected counts, and roundtrip a sample actor.

use std::path::PathBuf;
use std::time::Instant;

use scim_store::{
    actor::list_actors_in_snapshot, blob::read_blob, import::import_save,
    snapshot::list_snapshots, Db,
};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join(name)
}

#[test]
fn import_creative_test_sav_to_fresh_project() {
    let sav = corpus_path("CREATIVE TEST.sav");
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("creative.scimdb");
    let mut db = Db::open(&db_path).expect("open db");

    let t0 = Instant::now();
    let summary = import_save(&mut db, &sav, "CREATIVE TEST.sav").expect("import");
    let elapsed = t0.elapsed();

    eprintln!(
        "=== P1.4 import_corpus report ===\n  actors: {}\n  blobs:  {}\n  failed: {}\n  elapsed: {:.2}s",
        summary.total_actors,
        summary.blobs_inserted,
        summary.failed_actors,
        elapsed.as_secs_f64()
    );

    assert_eq!(summary.total_actors, 23941, "expected 23941 actors imported");
    assert_eq!(summary.failed_actors, 0, "no failed actors");
    assert!(summary.blobs_inserted > 0);
    assert!(summary.blobs_inserted <= summary.total_actors);

    assert!(
        elapsed.as_secs_f64() < 10.0,
        "import took {:.2}s; SQLite tuning may be off",
        elapsed.as_secs_f64()
    );

    let snaps = list_snapshots(db.conn()).unwrap();
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].label, "CREATIVE TEST.sav");

    let actors = list_actors_in_snapshot(db.conn(), summary.snapshot_id).unwrap();
    assert_eq!(actors.len(), summary.total_actors);

    let first = &actors[0];
    let plain = read_blob(db.conn(), first.blob_hash).unwrap().unwrap();
    assert!(!plain.is_empty(), "actor blob should be non-empty");

    drop(db);
    let db2 = Db::open(&db_path).unwrap();
    let snaps2 = list_snapshots(db2.conn()).unwrap();
    assert_eq!(snaps2.len(), 1);
}
