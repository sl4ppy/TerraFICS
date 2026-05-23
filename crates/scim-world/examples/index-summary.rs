//! `cargo run -p scim-world --example index-summary -- <save.sav>`
//!
//! Imports a save into a temporary project DB, materializes a `WorldIndex`,
//! and prints summary statistics + a sample viewport query.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use scim_store::{import::import_save, Db};
use scim_world::WorldIndex;

fn main() -> ExitCode {
    let Some(arg) = env::args().nth(1) else {
        eprintln!("usage: index-summary <save.sav>");
        return ExitCode::from(2);
    };
    let sav = PathBuf::from(arg);

    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("tempdir failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let db_path = dir.path().join("index-summary.scimdb");
    let mut db = match Db::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Db::open failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let t_import = Instant::now();
    let summary = match import_save(&mut db, &sav, "index-summary") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("import_save failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let import_secs = t_import.elapsed().as_secs_f64();

    let t_idx = Instant::now();
    let idx = match WorldIndex::from_snapshot(db.conn(), summary.snapshot_id) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("from_snapshot failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let idx_secs = t_idx.elapsed().as_secs_f64();

    let t_q = Instant::now();
    let near_origin: usize = idx
        .query_aabb([-50_000.0, -50_000.0], [50_000.0, 50_000.0])
        .count();
    let q_secs = t_q.elapsed().as_secs_f64();

    println!("=== scim-world index-summary ===");
    println!("  save:        {}", sav.display());
    println!("  total rows:  {}", summary.total_actors);
    println!("  index size:  {}", idx.len());
    println!("  import:      {import_secs:.3}s");
    println!("  build index: {idx_secs:.3}s");
    println!("  q ±50k:      {near_origin} hits in {q_secs:.6}s");
    ExitCode::SUCCESS
}
